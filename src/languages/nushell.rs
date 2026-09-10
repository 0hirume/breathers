use std::collections::BTreeSet;

use tree_sitter::Node;

use crate::configuration::{Rule, Rules};

fn opaque(node: Node<'_>) -> bool {
    matches!(
        node.kind(),
        "comment" | "shebang" | "val_string" | "val_interpolated"
    )
}

fn statement(mut node: Node<'_>) -> Node<'_> {
    while matches!(node.kind(), "pipeline" | "pipe_element") {
        let mut cursor = node.walk();

        let mut children = node
            .named_children(&mut cursor)
            .filter(|child| child.kind() != "comment");

        let Some(child) = children.next() else { break };

        if children.next().is_some() {
            break;
        }

        node = child;
    }

    node
}

fn block(node: Node<'_>) -> Option<Rule> {
    let node = statement(node);

    match node.kind() {
        "stmt_let" | "stmt_mut" | "stmt_const" => node.child_by_field_name("value").and_then(block),
        "ctrl_if" => Some(Rule::Conditionals),
        "ctrl_for" => Some(Rule::ForLoops),
        "ctrl_while" => Some(Rule::WhileLoops),
        "ctrl_loop" => Some(Rule::LoopStatements),
        "ctrl_match" => Some(Rule::Matches),
        "ctrl_try" => Some(Rule::TryBlocks),
        "decl_def" | "decl_extern" => Some(Rule::Functions),
        "decl_module" | "decl_export" => Some(Rule::Modules),
        _ => None,
    }
}

fn expression(node: Node<'_>) -> Option<Rule> {
    match node.kind() {
        "pipeline" => {
            let mut cursor = node.walk();

            (node
                .named_children(&mut cursor)
                .filter(|child| child.kind() == "pipe_element")
                .count()
                > 1)
            .then_some(Rule::Pipelines)
        }

        "command" | "where_command" => Some(Rule::Calls),
        "val_list" => Some(Rule::Arrays),
        "val_table" => Some(Rule::Tables),
        "val_record" => Some(Rule::Objects),
        _ => None,
    }
}

fn complex(node: Node<'_>, source: &str, rules: &Rules) -> bool {
    if let Some(rule) = block(node) {
        rules.enabled(rule)
    } else {
        let rule = crate::syntax::rule(node, expression, opaque).unwrap_or(
            if matches!(
                node.kind(),
                "stmt_let" | "stmt_mut" | "stmt_const" | "decl_alias"
            ) {
                Rule::Declarations
            } else {
                Rule::Multiline
            },
        );

        rules.enabled(rule) && crate::syntax::multiline(node, source, opaque)
    }
}

fn returns(node: Node<'_>, source: &str) -> bool {
    let node = statement(node);

    node.kind() == "command"
        && node
            .child_by_field_name("head")
            .is_some_and(|head| &source[node.start_byte()..head.end_byte()] == "return")
}

fn visit(node: Node<'_>, source: &str, rules: &Rules, insertions: &mut BTreeSet<usize>) {
    if opaque(node) {
        return;
    }

    let mut cursor = node.walk();
    let children: Vec<_> = node.named_children(&mut cursor).collect();

    if matches!(
        node.kind(),
        "nu_script" | "block" | "val_closure" | "expr_parenthesized" | "record_body" | "ctrl_match"
    ) {
        let mut previous = None;

        for (index, child) in children.iter().enumerate() {
            if matches!(
                child.kind(),
                "comment" | "shebang" | "parameter_pipes" | "cell_path"
            ) || (node.kind() == "ctrl_match"
                && !matches!(child.kind(), "match_arm" | "default_arm"))
            {
                continue;
            }

            if let Some(previous) = previous {
                let left: Node<'_> = children[previous];

                let separate = if matches!(node.kind(), "record_body" | "ctrl_match") {
                    let rule = if node.kind() == "record_body" {
                        Rule::RecordFields
                    } else {
                        Rule::MatchArms
                    };

                    rules.enabled(rule)
                        && (crate::syntax::multiline(left, source, opaque)
                            || crate::syntax::multiline(*child, source, opaque))
                } else {
                    complex(left, source, rules)
                        || complex(*child, source, rules)
                        || (rules.enabled(Rule::ReturnStatements) && returns(*child, source))
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
    crate::syntax::breathe(source, &tree_sitter_nu::LANGUAGE.into(), rules, visit)
}

#[cfg(test)]
mod tests {
    use crate::configuration::Rules;

    #[test]
    fn spaces_blocks_pipelines_and_returns() {
        let source = "def example [] {\n    let ready = true\n    # attached\n    if $ready {\n        print ready\n    }\n    ls\n    | where size > 0\n    | get name\n    return done\n}\n";

        let expected = source
            .replace("    # attached", "\n    # attached")
            .replace("    ls", "\n    ls")
            .replace("    return", "\n    return");

        for (source, expected) in [
            (source.to_owned(), expected.clone()),
            (expected.clone(), expected.clone()),
            (source.replace('\n', "\r\n"), expected.replace('\n', "\r\n")),
        ] {
            assert_eq!(
                super::breathe(&source, &Rules::default()).unwrap(),
                expected
            );
        }

        assert!(super::breathe("def broken [] {", &Rules::default()).is_err());
    }

    #[test]
    fn preserves_literals_compact_code_and_pipeline_stages() {
        for source in [
            "let text = 'first\nsecond'\nprint done\n",
            "let text = r#'first\nsecond'#\nprint done\n",
            "let text = $\"first\n($value)\"\nprint done\n",
            "let first = 1\nlet second = 2\n",
            "let ready = true; if $ready { print yes }; print done\n",
            "ls\n| where size > 0\n| get name\n",
            "#!/usr/bin/env nu\nif true { print yes }\n",
            "print first\n\n\nreturn done",
            "print first\n^return done\n",
        ] {
            assert_eq!(super::breathe(source, &Rules::default()).unwrap(), source);
        }
    }

    #[test]
    fn spaces_definitions_loops_records_and_match_arms() {
        for (source, expected) in [
            (
                "print first\ndef example [] { print yes }\nprint last\n",
                "print first\n\ndef example [] { print yes }\n\nprint last\n",
            ),
            (
                "let values = [1 2]\nfor value in $values { print $value }\nprint last\n",
                "let values = [1 2]\n\nfor value in $values { print $value }\n\nprint last\n",
            ),
            (
                "mut ready = true\nwhile $ready { $ready = false }\nloop { break }\n",
                "mut ready = true\n\nwhile $ready { $ready = false }\n\nloop { break }\n",
            ),
            (
                "print first\ntry { print yes } catch { print error }\nprint last\n",
                "print first\n\ntry { print yes } catch { print error }\n\nprint last\n",
            ),
            (
                "module example {\n    export def ready [] { true }\n}\nuse example ready\n",
                "module example {\n    export def ready [] { true }\n}\n\nuse example ready\n",
            ),
            (
                "let value = {\n    first: 1\n    second: [\n        2\n    ]\n    third: 3\n}\nprint $value\n",
                "let value = {\n    first: 1\n\n    second: [\n        2\n    ]\n\n    third: 3\n}\n\nprint $value\n",
            ),
            (
                "match $value {\n    1 => {\n        print first\n    }\n    _ => { print other }\n}\n",
                "match $value {\n    1 => {\n        print first\n    }\n\n    _ => { print other }\n}\n",
            ),
            (
                "let transform = { |value|\n    print $value\n    return $value\n}\nprint done\n",
                "let transform = { |value|\n    print $value\n\n    return $value\n}\n\nprint done\n",
            ),
        ] {
            let formatted = super::breathe(source, &Rules::default()).unwrap();
            assert_eq!(formatted, expected);

            assert_eq!(
                super::breathe(&formatted, &Rules::default()).unwrap(),
                formatted
            );
        }
    }

    #[test]
    fn related_distinguishes_reads_from_writes_and_literals() {
        let rules: Rules = toml::from_str("related = true").unwrap();

        for source in [
            "let values = [1 2]\nfor value in $values { print $value }\n",
            "let value = { name: ready }\nreturn $value.name\n",
            "let value = 1\nreturn $\"($value)\"\n",
            "mut value = 1\n$value += 2\nreturn $value\n",
            "let value = 1\nlet result = (\n    $value + 1\n)\n",
        ] {
            assert_eq!(super::breathe(source, &rules).unwrap(), source);
        }

        for source in [
            "let value = 1\nreturn value\n",
            "let value = 1\nreturn 'value'\n",
            "let value = 1\nreturn { value: 2 }\n",
            "let value = 1\nreturn {|| $value }\n",
            "let value = 1\nreturn $other.value\n",
        ] {
            assert_eq!(
                super::breathe(source, &rules).unwrap(),
                source.replace("\nreturn", "\n\nreturn")
            );
        }
    }

    #[test]
    fn applies_granular_rules_and_related_dependencies() {
        for (source, configuration, expected) in [
            (
                "let ready = true\nif $ready { print yes }\n",
                "related = true",
                "let ready = true\nif $ready { print yes }\n",
            ),
            (
                "let ready = true\n\nif $ready { print yes }\n",
                "related = true",
                "let ready = true\n\nif $ready { print yes }\n",
            ),
            (
                "let ready = true\nif true { print $ready }\n",
                "related = true",
                "let ready = true\n\nif true { print $ready }\n",
            ),
            (
                "print first\nif true { print yes }\nprint last\n",
                "blocks = false",
                "print first\nif true { print yes }\nprint last\n",
            ),
            (
                "print first\nif true { print yes }\n",
                "blocks = false\nconditionals = true",
                "print first\n\nif true { print yes }\n",
            ),
            (
                "print first\nls\n| get name\nprint last\n",
                "pipelines = false",
                "print first\nls\n| get name\nprint last\n",
            ),
            (
                "print first\nreturn done\n",
                "returns = false",
                "print first\nreturn done\n",
            ),
        ] {
            let rules: Rules = toml::from_str(configuration).unwrap();
            assert_eq!(super::breathe(source, &rules).unwrap(), expected);
        }
    }
}
