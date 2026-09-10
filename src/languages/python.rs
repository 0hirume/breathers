use std::collections::BTreeSet;

use tree_sitter::Node;

use crate::configuration::{Rule, Rules};

fn opaque(node: Node<'_>) -> bool {
    matches!(node.kind(), "comment" | "string" | "concatenated_string")
}

fn block(node: Node<'_>) -> Option<Rule> {
    match node.kind() {
        "if_statement" => Some(Rule::Conditionals),
        "for_statement" => Some(Rule::ForLoops),
        "while_statement" => Some(Rule::WhileLoops),
        "with_statement" => Some(Rule::WithBlocks),
        "try_statement" => Some(Rule::TryBlocks),
        "match_statement" => Some(Rule::Matches),
        "function_definition" => Some(Rule::Functions),
        "class_definition" => Some(Rule::Classes),

        "decorated_definition" => Some(
            if node
                .child_by_field_name("definition")
                .is_some_and(|definition| definition.kind() == "class_definition")
            {
                Rule::Classes
            } else {
                Rule::DecoratedFunctions
            },
        ),

        _ => None,
    }
}

fn expression(node: Node<'_>) -> Option<Rule> {
    match node.kind() {
        "call" => Some(Rule::Calls),

        "list" | "tuple" | "set" | "list_comprehension" | "set_comprehension" => Some(Rule::Arrays),

        "dictionary" | "dictionary_comprehension" => Some(Rule::Objects),
        _ => None,
    }
}

fn complex(node: Node<'_>, source: &str, rules: &Rules) -> bool {
    if let Some(rule) = block(node) {
        rules.enabled(rule)
    } else {
        let rule = if node.kind() == "type_alias_statement" {
            Rule::TypeAliases
        } else {
            crate::syntax::rule(node, expression, opaque).unwrap_or(
                if node.kind() == "expression_statement"
                    && node
                        .named_child(0)
                        .is_some_and(|child| child.kind() == "assignment")
                {
                    Rule::Declarations
                } else {
                    Rule::Multiline
                },
            )
        };

        rules.enabled(rule) && crate::syntax::multiline(node, source, opaque)
    }
}

fn visit(node: Node<'_>, source: &str, rules: &Rules, insertions: &mut BTreeSet<usize>) {
    if opaque(node) {
        return;
    }

    let mut cursor = node.walk();
    let children: Vec<_> = node.named_children(&mut cursor).collect();

    if matches!(node.kind(), "module" | "block" | "dictionary") {
        let mut previous = None;

        for (index, child) in children.iter().enumerate() {
            if child.kind() == "comment" {
                continue;
            }

            if let Some(previous) = previous {
                let left: Node<'_> = children[previous];

                let separate = if node.kind() == "dictionary" {
                    rules.enabled(Rule::DictionaryEntries)
                        && (crate::syntax::multiline(left, source, opaque)
                            || crate::syntax::multiline(*child, source, opaque))
                } else if left.kind() == "case_clause" && child.kind() == "case_clause" {
                    rules.enabled(Rule::MatchCases)
                        && (crate::syntax::multiline(left, source, opaque)
                            || crate::syntax::multiline(*child, source, opaque))
                } else {
                    complex(left, source, rules)
                        || complex(*child, source, rules)
                        || (rules.enabled(Rule::ReturnStatements)
                            && child.kind() == "return_statement")
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

            previous = Some(index);
        }
    }

    for child in children {
        visit(child, source, rules, insertions);
    }
}

pub fn breathe(source: &str, rules: &Rules) -> Result<String, String> {
    crate::syntax::breathe(source, &tree_sitter_python::LANGUAGE.into(), rules, visit)
}

#[cfg(test)]
mod tests {
    use crate::configuration::Rules;

    #[test]
    fn spaces_python_without_touching_indentation_or_strings() {
        let source = "@decorate\ndef example():\n    first = 1\n    second = 2  # trailing\n    # attached\n    if first < second:\n        work()\n    else:\n        other()\n    save()\n    return first\n";

        let expected = source
            .replace("# trailing\n", "# trailing\n\n")
            .replace("    save()", "\n    save()")
            .replace("    return", "\n    return");

        let rules = Rules::default();
        assert_eq!(super::breathe(source, &rules).unwrap(), expected);
        assert_eq!(super::breathe(&expected, &rules).unwrap(), expected);

        assert_eq!(
            super::breathe(&source.replace('\n', "\r\n"), &rules).unwrap(),
            expected.replace('\n', "\r\n")
        );

        for source in [
            "text = '''first\nsecond'''\nwork()\n",
            "text = f'''first {value}\nsecond'''\nwork()\n",
            "def example(): return 1\n",
        ] {
            assert_eq!(super::breathe(source, &rules).unwrap(), source);
        }

        assert!(super::breathe("def broken(:\n", &rules).is_err());
    }

    #[test]
    fn disables_blocks_without_multiline_overriding_the_toggle() {
        let source = "first = 1\nif first:\n    work()\nfinish()\n";
        let rules: Rules = toml::from_str("blocks = false").unwrap();
        assert_eq!(super::breathe(source, &rules).unwrap(), source);
    }
}
