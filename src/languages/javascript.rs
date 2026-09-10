use std::collections::BTreeSet;

use tree_sitter::{Language, Node, Parser};

use crate::configuration::{Rule, Rules};

fn opaque(node: Node<'_>) -> bool {
    matches!(
        node.kind(),
        "comment"
            | "html_comment"
            | "hash_bang_line"
            | "string"
            | "template_string"
            | "regex"
            | "jsx_text"
    )
}

fn block(node: Node<'_>) -> Option<Rule> {
    match node.kind() {
        "if_statement" => Some(Rule::Conditionals),
        "for_statement" | "for_in_statement" => Some(Rule::ForLoops),
        "while_statement" => Some(Rule::WhileLoops),
        "do_statement" | "statement_block" | "class_static_block" => Some(Rule::DoBlocks),
        "switch_statement" => Some(Rule::Switches),
        "try_statement" => Some(Rule::TryBlocks),
        "with_statement" => Some(Rule::WithBlocks),

        "function_declaration" | "generator_function_declaration" | "method_definition" => {
            Some(Rule::Functions)
        }

        "class_declaration" => Some(Rule::Classes),
        "interface_declaration" => Some(Rule::Interfaces),
        "export_statement" => node.child_by_field_name("declaration").and_then(block),
        _ => None,
    }
}

fn complex(node: Node<'_>, source: &str, rules: &Rules) -> bool {
    if let Some(rule) = block(node) {
        rules.enabled(rule)
    } else {
        let rule = crate::syntax::rule(
            node,
            |node| match node.kind() {
                "type_alias_declaration" => Some(Rule::TypeAliases),
                "call_expression" | "new_expression" => Some(Rule::Calls),
                "array" => Some(Rule::Arrays),
                "object" => Some(Rule::Objects),
                _ => None,
            },
            opaque,
        )
        .unwrap_or(
            if matches!(node.kind(), "lexical_declaration" | "variable_declaration") {
                Rule::Declarations
            } else {
                Rule::Multiline
            },
        );

        rules.enabled(rule) && crate::syntax::multiline(node, source, opaque)
    }
}

fn visit(node: Node<'_>, source: &str, rules: &Rules, insertions: &mut BTreeSet<usize>) {
    if opaque(node) || node.kind() == "jsx_element" {
        return;
    }

    let mut cursor = node.walk();
    let children: Vec<_> = node.named_children(&mut cursor).collect();

    if matches!(
        node.kind(),
        "program"
            | "statement_block"
            | "switch_body"
            | "switch_case"
            | "switch_default"
            | "class_body"
            | "interface_body"
            | "object_type"
            | "object"
            | "enum_body"
    ) {
        let mut previous = None;

        for (index, child) in children.iter().enumerate() {
            if matches!(child.kind(), "comment" | "html_comment" | "empty_statement")
                || Some(*child) == node.child_by_field_name("value")
            {
                continue;
            }

            if let Some(previous) = previous {
                let left: Node<'_> = children[previous];

                let separate = match node.kind() {
                    "switch_body" => {
                        rules.enabled(Rule::SwitchCases)
                            && (crate::syntax::multiline(left, source, opaque)
                                || crate::syntax::multiline(*child, source, opaque))
                    }

                    "enum_body" => {
                        rules.enabled(Rule::EnumMembers)
                            && (crate::syntax::multiline(left, source, opaque)
                                || crate::syntax::multiline(*child, source, opaque))
                    }

                    "class_body" | "interface_body" | "object_type" | "object" => {
                        let rule = match node.kind() {
                            "class_body" => Rule::ClassMembers,
                            "interface_body" => Rule::InterfaceMembers,
                            "object_type" => Rule::TypeMembers,
                            _ => Rule::ObjectProperties,
                        };

                        rules.enabled(rule)
                            && (crate::syntax::multiline(left, source, opaque)
                                || crate::syntax::multiline(*child, source, opaque))
                    }

                    _ => {
                        complex(left, source, rules)
                            || complex(*child, source, rules)
                            || (rules.enabled(Rule::ReturnStatements)
                                && child.kind() == "return_statement")
                    }
                };

                if left.kind() != "hash_bang_line"
                    && separate
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

            previous = Some(index);
        }
    }

    for child in children {
        visit(child, source, rules, insertions);
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
    visit(tree.root_node(), source, rules, &mut insertions);

    Ok(crate::spacing::apply(source, insertions))
}

#[cfg(test)]
mod tests {
    use crate::configuration::Rules;

    #[test]
    fn spaces_javascript_typescript_and_tsx() {
        let source = "function example() {\n    const first = 1;\n    const second = 2; // trailing\n    // attached\n    if (first < second) {\n        work();\n    } else {\n        other();\n    }\n    save();\n    return first;\n}\n";

        let expected = source
            .replace("// trailing\n", "// trailing\n\n")
            .replace("    save();", "\n    save();")
            .replace("    return", "\n    return");

        for language in [
            tree_sitter_javascript::LANGUAGE.into(),
            tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            tree_sitter_typescript::LANGUAGE_TSX.into(),
        ] {
            assert_eq!(
                super::breathe(source, &language, &Rules::default()).unwrap(),
                expected
            );

            assert_eq!(
                super::breathe(&expected, &language, &Rules::default()).unwrap(),
                expected
            );

            assert_eq!(
                super::breathe(&source.replace('\n', "\r\n"), &language, &Rules::default())
                    .unwrap(),
                expected.replace('\n', "\r\n")
            );
        }
    }

    #[test]
    fn preserves_literals_and_selects_typescript_grammars() {
        for source in [
            "const text = `first\nsecond`;\nfinish();\n",
            "const element = <div>first\nsecond</div>;\nfinish();\n",
            "const pattern = /foo[{}]/;\nfinish();\n",
        ] {
            assert_eq!(
                super::breathe(
                    source,
                    &tree_sitter_javascript::LANGUAGE.into(),
                    &Rules::default()
                )
                .unwrap(),
                source
            );
        }

        let source = "const value = <number>result;\nfinish();\n";

        assert_eq!(
            super::breathe(
                source,
                &tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
                &Rules::default()
            )
            .unwrap(),
            source
        );

        assert!(
            super::breathe(
                source,
                &tree_sitter_typescript::LANGUAGE_TSX.into(),
                &Rules::default()
            )
            .is_err()
        );

        let source = "const element: Element = <div>text</div>;\nfinish();\n";

        assert_eq!(
            super::breathe(
                source,
                &tree_sitter_typescript::LANGUAGE_TSX.into(),
                &Rules::default()
            )
            .unwrap(),
            source
        );

        assert!(
            super::breathe(
                "function broken( {",
                &tree_sitter_javascript::LANGUAGE.into(),
                &Rules::default()
            )
            .is_err()
        );
    }
}
