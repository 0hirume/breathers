use std::collections::BTreeSet;

use tree_sitter::{Language, Node};

use crate::{
    configuration::{Rule, Rules},
    error::Failure,
    syntax::{self, Pair, Policy, multiline, rule},
};

pub(crate) fn breathe(source: &str, language: &Language, rules: &Rules) -> Result<String, Failure> {
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
        && !(rules.enabled(Rule::Related) && related(left, right, source))
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

fn related(previous: Node<'_>, next: Node<'_>, source: &str) -> bool {
    let mut names = BTreeSet::new();
    written(previous, source, &mut names);

    if let Some(initializer) = next.child_by_field_name("initializer") {
        let mut shadowed = BTreeSet::new();

        if matches!(
            initializer.kind(),
            "lexical_declaration" | "variable_declaration"
        ) {
            written(initializer, source, &mut shadowed);
        }

        names.retain(|name| !shadowed.contains(name));
    }

    !names.is_empty() && reads(next, source, &names)
}

fn written<'source>(node: Node<'_>, source: &'source str, names: &mut BTreeSet<&'source str>) {
    let field = match node.kind() {
        "variable_declarator" => Some("name"),
        "assignment_expression" | "augmented_assignment_expression" => Some("left"),
        "update_expression" => Some("argument"),
        _ => None,
    };

    if let Some(target) = field.and_then(|field| node.child_by_field_name(field)) {
        bindings(target, source, names);

        if let Some(right) = node.child_by_field_name("right") {
            written(right, source, names);
        }
    } else if matches!(
        node.kind(),
        "expression_statement" | "variable_declaration" | "lexical_declaration"
    ) {
        let mut cursor = node.walk();

        for child in node.named_children(&mut cursor) {
            written(child, source, names);
        }
    }
}

fn bindings<'source>(node: Node<'_>, source: &'source str, names: &mut BTreeSet<&'source str>) {
    match node.kind() {
        "identifier" | "shorthand_property_identifier_pattern" => {
            names.insert(&source[node.byte_range()]);
        }

        "pair_pattern" => {
            if let Some(value) = node.child_by_field_name("value") {
                bindings(value, source, names);
            }
        }

        "assignment_pattern" => {
            if let Some(left) = node.child_by_field_name("left") {
                bindings(left, source, names);
            }
        }

        "member_expression" | "subscript_expression" => {
            if let Some(object) = node.child_by_field_name("object") {
                bindings(object, source, names);
            }
        }

        "object_pattern" | "array_pattern" | "rest_pattern" | "parenthesized_expression" => {
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
        "identifier" | "shorthand_property_identifier_pattern" => false,
        "member_expression" | "subscript_expression" => reads(node, source, names),

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
        "identifier" | "shorthand_property_identifier" => {
            names.contains(&source[node.byte_range()])
        }

        "comment"
        | "regex"
        | "hash_bang_line"
        | "function_declaration"
        | "generator_function_declaration"
        | "function_expression"
        | "arrow_function"
        | "class_declaration"
        | "class"
        | "method_definition"
        | "statement_block"
        | "class_body"
        | "interface_body"
        | "else_clause"
        | "catch_clause"
        | "finally_clause"
        | "type_annotation"
        | "type_arguments"
        | "type_parameters"
        | "type_alias_declaration"
        | "jsx_closing_element"
        | "jsx_namespace_name"
        | "import_statement"
        | "string" => false,

        "jsx_opening_element" | "jsx_self_closing_element" => {
            let mut cursor = node.walk();

            node.named_children(&mut cursor).any(|child| {
                !(Some(child) == node.child_by_field_name("name")
                    && child.kind() == "identifier"
                    && source[child.byte_range()]
                        .starts_with(|character: char| character.is_ascii_lowercase()))
                    && reads(child, source, names)
            })
        }

        "member_expression" => field("object"),
        "variable_declarator" => field("value"),

        "assignment_expression" => {
            field("right")
                || node
                    .child_by_field_name("left")
                    .is_some_and(|left| target_reads(left, source, names))
                || (node
                    .child_by_field_name("operator")
                    .is_some_and(|operator| &source[operator.byte_range()] != "=")
                    && field("left"))
        }

        "augmented_assignment_expression" => field("right") || field("left"),
        "for_in_statement" => field("right"),

        _ => {
            let mut cursor = node.walk();

            node.named_children(&mut cursor).any(|child| {
                !["type", "return_type", "alias", "label"]
                    .into_iter()
                    .any(|field| node.child_by_field_name(field) == Some(child))
                    && reads(child, source, names)
            })
        }
    }
}
