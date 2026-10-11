use tree_sitter::{Language, Node};

use super::dependencies;

use crate::{
    configuration::{Rule, Rules},
    syntax::{self, Pair, Policy, multiline, rule},
};

pub(crate) fn breathe(source: &str, language: &Language, rules: &Rules) -> Result<String, String> {
    let policy = Policy {
        opaque: terminal,
        container,
        member,
        separate,
        trivia: |_| true,
        continuation: None,
    };

    syntax::breathe(source, language, rules, &policy, &[])
}

fn terminal(node: Node<'_>) -> bool {
    opaque(node) || node.kind() == "jsx_element"
}

fn container(node: Node<'_>) -> bool {
    matches!(
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
    )
}

fn member(node: Node<'_>, parent: Node<'_>) -> bool {
    !matches!(node.kind(), "comment" | "html_comment" | "empty_statement")
        && Some(node) != parent.child_by_field_name("value")
}

fn separate(pair: Pair<'_>, source: &str, rules: &Rules) -> bool {
    let Pair {
        parent,
        left,
        right,
        ..
    } = pair;

    let separate = match parent.kind() {
        "switch_body" | "enum_body" | "class_body" | "interface_body" | "object_type"
        | "object" => {
            let rule = match parent.kind() {
                "switch_body" => Rule::SwitchCases,
                "enum_body" => Rule::EnumMembers,
                "class_body" => Rule::ClassMembers,
                "interface_body" => Rule::InterfaceMembers,
                "object_type" => Rule::TypeMembers,
                _ => Rule::ObjectProperties,
            };

            rules.enabled(rule)
                && (multiline(left, source, opaque) || multiline(right, source, opaque))
        }

        _ => {
            complex(left, source, rules)
                || complex(right, source, rules)
                || (rules.enabled(Rule::ReturnStatements) && right.kind() == "return_statement")
        }
    };

    left.kind() != "hash_bang_line"
        && separate
        && !(rules.enabled(Rule::Related) && dependencies::related(left, right, source))
}

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

fn expression(node: Node<'_>) -> Option<Rule> {
    match node.kind() {
        "type_alias_declaration" => Some(Rule::TypeAliases),
        "call_expression" | "new_expression" => Some(Rule::Calls),
        "array" => Some(Rule::Arrays),
        "object" => Some(Rule::Objects),
        _ => None,
    }
}

fn complex(node: Node<'_>, source: &str, rules: &Rules) -> bool {
    if let Some(rule) = block(node) {
        rules.enabled(rule)
    } else {
        let rule = rule(node, expression, opaque).unwrap_or(
            if matches!(node.kind(), "lexical_declaration" | "variable_declaration") {
                Rule::Declarations
            } else {
                Rule::Multiline
            },
        );

        rules.enabled(rule) && multiline(node, source, opaque)
    }
}
