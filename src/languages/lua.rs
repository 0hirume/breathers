use std::collections::BTreeSet;

use tree_sitter::Node;

use crate::{
    configuration::{Rule, Rules},
    error::Failure,
    syntax::{self, Pair, Policy, multiline, rule},
};

pub(crate) fn breathe(source: &str, rules: &Rules) -> Result<String, Failure> {
    let policy = Policy {
        opaque,
        container,
        member,
        separate,
        trivia: |_| true,
        continuation: None,
    };

    syntax::breathe(
        source,
        &tree_sitter_lua::LANGUAGE.into(),
        rules,
        &policy,
        &[],
    )
}

fn container(node: Node<'_>) -> bool {
    matches!(node.kind(), "chunk" | "block")
}

fn member(node: Node<'_>, _: Node<'_>) -> bool {
    !matches!(node.kind(), "comment" | "empty_statement")
}

fn separate(pair: Pair<'_>, source: &str, rules: &Rules) -> bool {
    let Pair { left, right, .. } = pair;

    left.kind() != "hash_bang_line"
        && (complex(left, source, rules)
            || complex(right, source, rules)
            || (rules.enabled(Rule::ReturnStatements) && right.kind() == "return_statement"))
        && !(rules.enabled(Rule::Related) && related(left, right, source))
}

fn opaque(node: Node<'_>) -> bool {
    matches!(node.kind(), "string" | "comment" | "hash_bang_line")
}

fn block(node: Node<'_>) -> Option<Rule> {
    match node.kind() {
        "if_statement" => Some(Rule::Conditionals),
        "while_statement" => Some(Rule::WhileLoops),
        "repeat_statement" => Some(Rule::RepeatLoops),
        "for_statement" => Some(Rule::ForLoops),
        "do_statement" => Some(Rule::DoBlocks),
        "function_declaration" | "function_definition" => Some(Rule::Functions),
        _ => None,
    }
}

fn expression(node: Node<'_>) -> Option<Rule> {
    match node.kind() {
        "function_call" => Some(Rule::Calls),
        "table_constructor" => Some(Rule::Tables),
        _ => None,
    }
}

fn complex(node: Node<'_>, source: &str, rules: &Rules) -> bool {
    if let Some(rule) = block(node) {
        rules.enabled(rule)
    } else {
        let rule =
            rule(node, expression, opaque).unwrap_or(if node.kind() == "variable_declaration" {
                Rule::Declarations
            } else {
                Rule::Multiline
            });

        rules.enabled(rule) && multiline(node, source, opaque)
    }
}

fn related(previous: Node<'_>, next: Node<'_>, source: &str) -> bool {
    let mut names = BTreeSet::new();
    written(previous, source, &mut names);

    !names.is_empty() && reads(next, source, &names)
}

fn written<'source>(node: Node<'_>, source: &'source str, names: &mut BTreeSet<&'source str>) {
    if matches!(node.kind(), "variable_declaration" | "assignment_statement") {
        let mut cursor = node.walk();

        for child in node.named_children(&mut cursor) {
            if child.kind() == "variable_list" {
                bindings(child, source, names);
            } else {
                written(child, source, names);
            }
        }
    }
}

fn bindings<'source>(node: Node<'_>, source: &'source str, names: &mut BTreeSet<&'source str>) {
    match node.kind() {
        "identifier" => {
            names.insert(&source[node.byte_range()]);
        }

        "dot_index_expression" | "bracket_index_expression" => {
            if let Some(table) = node.child_by_field_name("table") {
                bindings(table, source, names);
            }
        }

        "variable_list" | "parenthesized_expression" => {
            let mut cursor = node.walk();

            for child in node.named_children(&mut cursor) {
                bindings(child, source, names);
            }
        }

        _ => {}
    }
}

fn target_reads(node: Node<'_>, source: &str, names: &BTreeSet<&str>) -> bool {
    match node.kind() {
        "identifier" => false,
        "dot_index_expression" | "bracket_index_expression" => reads(node, source, names),

        _ => {
            let mut cursor = node.walk();

            node.named_children(&mut cursor)
                .any(|child| target_reads(child, source, names))
        }
    }
}

fn reads(node: Node<'_>, source: &str, names: &BTreeSet<&str>) -> bool {
    let field = |field| {
        node.child_by_field_name(field)
            .is_some_and(|child| reads(child, source, names))
    };

    match node.kind() {
        "identifier" => names.contains(&source[node.byte_range()]),

        "comment"
        | "hash_bang_line"
        | "function_declaration"
        | "function_definition"
        | "block"
        | "else_statement"
        | "elseif_statement"
        | "variable_list"
        | "attribute"
        | "string" => false,

        "dot_index_expression" | "method_index_expression" => field("table"),

        "field" => {
            field("value")
                || (node.child(0).is_some_and(|child| child.kind() == "[") && field("name"))
        }

        "for_numeric_clause" => field("start") || field("end") || field("step"),

        "assignment_statement" => {
            let mut cursor = node.walk();

            node.named_children(&mut cursor).any(|child| {
                if child.kind() == "variable_list" {
                    target_reads(child, source, names)
                } else {
                    reads(child, source, names)
                }
            })
        }

        _ => {
            let mut cursor = node.walk();

            node.named_children(&mut cursor)
                .any(|child| reads(child, source, names))
        }
    }
}
