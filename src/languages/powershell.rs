use std::collections::BTreeSet;

use tree_sitter::Node;

use crate::configuration::{Rule, Rules};

fn opaque(node: Node<'_>) -> bool {
    matches!(
        node.kind(),
        "comment"
            | "string_literal"
            | "expandable_string_literal"
            | "expandable_here_string_literal"
            | "verbatim_string_characters"
            | "verbatim_here_string_characters"
    )
}

fn statement(mut node: Node<'_>) -> Node<'_> {
    while matches!(node.kind(), "pipeline" | "assignment_value") {
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
        "assignment_expression" => node.child_by_field_name("value").and_then(block),
        "if_statement" => Some(Rule::Conditionals),
        "for_statement" | "foreach_statement" => Some(Rule::ForLoops),
        "while_statement" => Some(Rule::WhileLoops),
        "do_statement" => Some(Rule::DoBlocks),
        "switch_statement" => Some(Rule::Switches),
        "try_statement" | "trap_statement" => Some(Rule::TryBlocks),
        "function_statement" => Some(Rule::Functions),
        "class_statement" | "enum_statement" => Some(Rule::Classes),

        "named_block"
        | "data_statement"
        | "inlinescript_statement"
        | "parallel_statement"
        | "sequence_statement" => Some(Rule::Blocks),

        _ => None,
    }
}

fn expression(node: Node<'_>) -> Option<Rule> {
    match node.kind() {
        "pipeline" | "assignment_value" | "pipeline_chain" => {
            let mut cursor = node.walk();

            node.children(&mut cursor)
                .any(|child| matches!(child.kind(), "|" | "pipeline_chain_tail"))
                .then_some(Rule::Pipelines)
        }

        "command"
        | "invocation_expression"
        | "invocation_foreach_expression"
        | "invocation_where_expression" => Some(Rule::Calls),

        "array_expression" | "array_literal_expression" => Some(Rule::Arrays),
        "hash_literal_expression" => Some(Rule::Objects),
        _ => None,
    }
}

fn complex(node: Node<'_>, source: &str, rules: &Rules) -> bool {
    if let Some(rule) = block(node) {
        rules.enabled(rule)
    } else {
        let fallback = if statement(node).kind() == "assignment_expression" {
            Rule::Declarations
        } else {
            Rule::Multiline
        };

        let rule = crate::syntax::rule(node, expression, opaque).unwrap_or(fallback);

        rules.enabled(rule) && crate::syntax::multiline(node, source, opaque)
    }
}

fn variable(source: &str) -> impl Iterator<Item = char> + '_ {
    let source = source.trim_start_matches(['$', '@']);

    let source = source
        .strip_prefix('{')
        .and_then(|source| source.strip_suffix('}'))
        .unwrap_or(source);

    let mut characters = source.chars();

    std::iter::from_fn(move || match characters.next()? {
        '`' => characters.next(),
        character => Some(character),
    })
    .flat_map(char::to_lowercase)
}

fn reads(node: Node<'_>, target: Node<'_>, source: &str) -> bool {
    match node.kind() {
        "variable" => {
            variable(&source[node.byte_range()]).eq(variable(&source[target.byte_range()]))
        }

        "comment"
        | "verbatim_string_characters"
        | "verbatim_here_string_characters"
        | "function_statement"
        | "class_statement"
        | "script_block_expression"
        | "statement_block"
        | "type_literal" => false,

        "assignment_expression" => {
            node.child_by_field_name("value")
                .is_some_and(|value| reads(value, target, source))
                || (node
                    .named_child(1)
                    .is_some_and(|operator| &source[operator.byte_range()] != "=")
                    && node
                        .named_child(0)
                        .is_some_and(|left| reads(left, target, source)))
        }

        _ => {
            let mut cursor = node.walk();

            node.named_children(&mut cursor)
                .any(|child| reads(child, target, source))
        }
    }
}

fn dependencies(target: Node<'_>, next: Node<'_>, source: &str) -> bool {
    if target.kind() == "variable" {
        return reads(next, target, source);
    }

    if matches!(target.kind(), "member_access" | "element_access") {
        return target
            .named_child(0)
            .is_some_and(|receiver| dependencies(receiver, next, source));
    }

    let mut cursor = target.walk();

    target
        .named_children(&mut cursor)
        .any(|child| dependencies(child, next, source))
}

fn related(previous: Node<'_>, next: Node<'_>, source: &str) -> bool {
    let previous = statement(previous);

    previous.kind() == "assignment_expression"
        && previous
            .named_child(0)
            .is_some_and(|target| dependencies(target, next, source))
}

fn returns(node: Node<'_>, source: &str) -> bool {
    node.kind() == "flow_control_statement"
        && node
            .child(0)
            .is_some_and(|keyword| source[keyword.byte_range()].eq_ignore_ascii_case("return"))
}

fn member(node: Node<'_>, parent: Node<'_>) -> bool {
    match parent.kind() {
        "class_statement" => matches!(
            node.kind(),
            "class_property_definition" | "class_method_definition"
        ),

        "enum_statement" => node.kind() == "enum_member",
        _ => !matches!(node.kind(), "comment" | "empty_statement" | "label"),
    }
}

fn visit(node: Node<'_>, source: &str, rules: &Rules, insertions: &mut BTreeSet<usize>) {
    if opaque(node) {
        return;
    }

    let container = match node.kind() {
        "statement_list" | "named_block_list" => Some(None),
        "hash_literal_body" => Some(Some(Rule::DictionaryEntries)),
        "switch_clauses" => Some(Some(Rule::SwitchCases)),
        "class_statement" => Some(Some(Rule::ClassMembers)),
        "enum_statement" => Some(Some(Rule::EnumMembers)),
        _ => None,
    };

    let mut previous: Option<Node<'_>> = None;
    let mut cursor = node.walk();

    for child in node.named_children(&mut cursor) {
        if let Some(rule) = container
            && member(child, node)
        {
            if let Some(left) = previous {
                let separate = if let Some(rule) = rule {
                    rules.enabled(rule)
                        && (crate::syntax::multiline(left, source, opaque)
                            || crate::syntax::multiline(child, source, opaque))
                } else {
                    complex(left, source, rules)
                        || complex(child, source, rules)
                        || (rules.enabled(Rule::ReturnStatements) && returns(child, source))
                };

                if separate
                    && !(rule.is_none()
                        && rules.enabled(Rule::Related)
                        && related(left, child, source))
                    && let Some(offset) = crate::syntax::boundary_ranges(
                        source,
                        left.end_byte(),
                        child.start_byte(),
                        std::iter::successors(left.next_named_sibling(), |node| {
                            node.next_named_sibling()
                        })
                        .take_while(|node| node.id() != child.id())
                        .filter(|node| node.kind() == "comment")
                        .map(|node| node.byte_range()),
                    )
                    && !source[..offset]
                        .trim_end_matches(['\r', '\n'])
                        .ends_with('`')
                {
                    insertions.insert(offset);
                }
            }

            previous = Some(child);
        }

        visit(child, source, rules, insertions);
    }
}

pub fn breathe(source: &str, rules: &Rules) -> Result<String, String> {
    crate::syntax::breathe(source, &tree_sitter_pwsh::LANGUAGE.into(), rules, visit)
}

#[cfg(test)]
mod tests {
    use crate::configuration::{Configuration, Rules};

    #[test]
    fn preserves_powershell_syntax_and_spacing_boundaries() {
        let source = "#Requires -Version 7.0\nusing namespace System.IO\nfunction Get-Value {\n    param([string]$Path)\n    begin { Write-Output ready }\n    process {\n        $value = Get-Item $Path\n        # attached\n        if ($value) {\n            Write-Output $value\n        }\n        Get-Process\n        | Sort-Object CPU\n        RETURN $value\n    }\n    clean { Remove-Variable value }\n}\nWrite-Output done\n";

        let expected = source
            .replace("    process", "\n    process")
            .replace("        # attached", "\n        # attached")
            .replace("        Get-Process", "\n        Get-Process")
            .replace("        RETURN", "\n        RETURN")
            .replace("    clean", "\n    clean")
            .replace("}\nWrite-Output done", "}\n\nWrite-Output done");

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

        for source in [
            "$text = @'\nfirst\nif ($true) { text }\n'@\nWrite-Output done\n",
            "$text = @\"\nfirst\n$(if ($true) { Write-Output text })\n\"@\nWrite-Output done\n",
            "$text = \"first\n$(Write-Output text; return 1)\"\nWrite-Output done\n",
            "Get-Process `\n    | Sort-Object CPU\n",
            "Get-Process\n| Sort-Object CPU\n",
            "Write-Output first; if ($true) { Write-Output yes }; return 1\n",
            "$result = $null ?? ($true ? 1 : 2)\nWrite-Output $result\n",
            "Write-Output first\n\n\nreturn 1",
        ] {
            assert_eq!(super::breathe(source, &Rules::default()).unwrap(), source);
        }

        assert!(super::breathe("function Broken {", &Rules::default()).is_err());

        let configuration: Configuration =
            toml::from_str("[powershell]\nrelated = true\n").unwrap();

        let source = "$Value = @(\n    1\n)\nWrite-Output \"${value}\"\nWrite-Output done\n";

        assert_eq!(
            super::breathe(source, &configuration.powershell).unwrap(),
            source
        );

        assert_eq!(
            super::breathe(source, &Rules::default()).unwrap(),
            source.replace(")\nWrite-Output", ")\n\nWrite-Output")
        );
    }
}
