use std::collections::BTreeSet;

use crate::configuration::{Rule, Rules};

use tree_sitter::{Language, Node, Parser};

fn opaque(node: Node<'_>) -> bool {
    matches!(
        node.kind(),
        "comment" | "string_literal" | "raw_string_literal" | "char_literal"
    ) || (node.kind().starts_with("preproc_")
        && !matches!(
            node.kind(),
            "preproc_if" | "preproc_ifdef" | "preproc_else" | "preproc_elif" | "preproc_elifdef"
        ))
}

fn multiline(node: Node<'_>, source: &str) -> bool {
    crate::syntax::multiline(node, source, opaque)
}

fn complex(node: Node<'_>, source: &str, rules: &Rules) -> bool {
    let block = match node.kind() {
        "if_statement" => Some(Rule::Conditionals),
        "switch_statement" => Some(Rule::Switches),
        "for_statement" | "for_range_loop" => Some(Rule::ForLoops),
        "while_statement" => Some(Rule::WhileLoops),
        "do_statement" | "compound_statement" => Some(Rule::DoBlocks),
        "try_statement" => Some(Rule::TryBlocks),
        "function_definition" => Some(Rule::Functions),
        _ => None,
    };

    if let Some(rule) = block {
        rules.enabled(rule)
    } else {
        let rule = crate::syntax::rule(
            node,
            |node| match node.kind() {
                "call_expression" => Some(Rule::Calls),
                "initializer_list" => Some(Rule::Arrays),
                _ => None,
            },
            opaque,
        )
        .unwrap_or(if node.kind() == "declaration" {
            Rule::Declarations
        } else {
            Rule::Multiline
        });

        rules.enabled(rule) && multiline(node, source)
    }
}

fn visit(node: Node<'_>, source: &str, insertions: &mut BTreeSet<usize>, rules: &Rules) {
    if opaque(node) || node.kind() == "call_expression" {
        return;
    }

    let mut cursor = node.walk();
    let children: Vec<_> = node.named_children(&mut cursor).collect();

    if matches!(
        node.kind(),
        "compound_statement"
            | "case_statement"
            | "field_declaration_list"
            | "enumerator_list"
            | "translation_unit"
            | "declaration_list"
    ) {
        let mut previous = None;

        for (index, child) in children.iter().enumerate() {
            if child.kind() == "comment" || Some(*child) == node.child_by_field_name("value") {
                continue;
            }

            if let Some(previous) = previous {
                let left: Node<'_> = children[previous];

                if !left.kind().starts_with("preproc_")
                    && !child.kind().starts_with("preproc_")
                    && left.kind() != "access_specifier"
                    && child.kind() != "access_specifier"
                {
                    let separate =
                        if matches!(node.kind(), "field_declaration_list" | "enumerator_list") {
                            let rule = if node.kind() == "enumerator_list" {
                                Rule::EnumMembers
                            } else if node
                                .parent()
                                .is_some_and(|parent| parent.kind() == "class_specifier")
                            {
                                Rule::ClassMembers
                            } else {
                                Rule::StructFields
                            };

                            rules.enabled(rule)
                                && (multiline(left, source) || multiline(*child, source))
                        } else if left.kind() == "case_statement"
                            && child.kind() == "case_statement"
                        {
                            rules.enabled(Rule::SwitchCases)
                                && (multiline(left, source) || multiline(*child, source))
                        } else {
                            complex(left, source, rules)
                                || complex(*child, source, rules)
                                || match child.kind() {
                                    "return_statement" => rules.enabled(Rule::ReturnStatements),
                                    "co_return_statement" => rules.enabled(Rule::CoroutineReturns),
                                    _ => false,
                                }
                        };

                    if separate
                        && !(rules.enabled(Rule::Related)
                            && crate::syntax::related(left, *child, source))
                        && let Some(offset) = crate::syntax::boundary(
                            source,
                            left.end_byte(),
                            child.start_byte(),
                            &children[previous + 1..index],
                        )
                    {
                        insertions.insert(offset);
                    }
                }
            }

            previous = Some(index);
        }
    }

    for child in children {
        visit(child, source, insertions, rules);
    }
}

pub fn breathe(source: &str, language: &Language, rules: &Rules) -> Result<String, String> {
    let mut parser = Parser::new();

    parser
        .set_language(language)
        .map_err(|error| error.to_string())?;

    let tree = parser.parse(source, None).ok_or("Could not parse source")?;

    if tree.root_node().has_error() {
        return Err("Syntax errors; no changes written".into());
    }

    let mut insertions = BTreeSet::new();
    visit(tree.root_node(), source, &mut insertions, rules);

    Ok(crate::spacing::apply(source, insertions))
}

#[cfg(test)]
mod tests {
    fn breathe(source: &str, language: &tree_sitter::Language) -> Result<String, String> {
        super::breathe(source, language, &crate::configuration::Rules::default())
    }

    #[test]
    fn spaces_c_and_cplusplus_statements() {
        let source = "int example(void) {\n    int first = 1;\n    int second = 2; // trailing\n    // attached\n    if (first < second) {\n        work();\n    }\n    save();\n    return first;\n}\n";

        let expected = source
            .replace("// trailing\n", "// trailing\n\n")
            .replace("    save();", "\n    save();")
            .replace("    return", "\n    return");

        for language in [
            tree_sitter_c::LANGUAGE.into(),
            tree_sitter_cpp::LANGUAGE.into(),
        ] {
            assert_eq!(breathe(source, &language).unwrap(), expected);
            assert_eq!(breathe(&expected, &language).unwrap(), expected);

            assert_eq!(
                breathe(&source.replace('\n', "\r\n"), &language).unwrap(),
                expected.replace('\n', "\r\n")
            );

            assert!(breathe("int broken( {", &language).is_err());
        }
    }

    #[test]
    fn preserves_directives_literals_and_macro_arguments() {
        for source in [
            "#define EXAMPLE(x) do { first(); second(); } while (0)\n",
            "#define EXAMPLE(x) \\\n    first(); \\\n    second();\n",
            "void example() {\n    MACRO([] {\n        first();\n        return second();\n    });\n    finish();\n}\n",
            "void example() {\n    auto text = R\"(first\nsecond)\";\n    finish();\n}\n",
            "void example() { first(); if (ready()) { second(); } third(); }\n",
        ] {
            let formatted = breathe(source, &tree_sitter_cpp::LANGUAGE.into()).unwrap();

            if source.contains("MACRO") {
                assert_eq!(
                    formatted,
                    source.replace("    finish();", "\n    finish();")
                );
            } else {
                assert_eq!(formatted, source);
            }
        }
    }

    #[test]
    fn spaces_fields_cases_and_guarded_code() {
        let source = "#ifndef EXAMPLE_H\n#define EXAMPLE_H\nint example(void) {\n    work();\n    return 1;\n}\n#endif\n";

        assert_eq!(
            breathe(source, &tree_sitter_c::LANGUAGE.into()).unwrap(),
            source.replace("    return", "\n    return")
        );

        let source = "struct Example {\n    int first;\n    int (*callback)(\n        int,\n        int\n    );\n    int last;\n};\n";

        let expected = source
            .replace("    int (*", "\n    int (*")
            .replace("    int last", "\n    int last");

        assert_eq!(
            breathe(source, &tree_sitter_c::LANGUAGE.into()).unwrap(),
            expected
        );

        let source = "void example(int value) {\n    switch (value) {\n        case 1:\n            work();\n            break;\n        default:\n            finish();\n            break;\n    }\n}\n";
        let expected = source.replace("        default:", "\n        default:");

        assert_eq!(
            breathe(source, &tree_sitter_cpp::LANGUAGE.into()).unwrap(),
            expected
        );
    }
}
