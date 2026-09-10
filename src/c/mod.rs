use std::collections::BTreeSet;

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

fn complex(node: Node<'_>, source: &str) -> bool {
    matches!(
        node.kind(),
        "if_statement"
            | "switch_statement"
            | "for_statement"
            | "for_range_loop"
            | "while_statement"
            | "do_statement"
            | "compound_statement"
            | "try_statement"
            | "function_definition"
    ) || multiline(node, source)
}

fn visit(node: Node<'_>, source: &str, insertions: &mut BTreeSet<usize>) {
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
                            multiline(left, source) || multiline(*child, source)
                        } else {
                            complex(left, source)
                                || complex(*child, source)
                                || matches!(
                                    child.kind(),
                                    "return_statement" | "co_return_statement"
                                )
                        };

                    if separate
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
        visit(child, source, insertions);
    }
}

pub fn breathe(source: &str, language: &Language) -> Result<String, String> {
    let mut parser = Parser::new();

    parser
        .set_language(language)
        .map_err(|error| error.to_string())?;

    let tree = parser.parse(source, None).ok_or("Could not parse source")?;

    if tree.root_node().has_error() {
        return Err("Syntax errors; no changes written".into());
    }

    let mut insertions = BTreeSet::new();
    visit(tree.root_node(), source, &mut insertions);

    Ok(crate::spacing::apply(source, insertions))
}

#[cfg(test)]
mod tests {
    use super::breathe;

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
