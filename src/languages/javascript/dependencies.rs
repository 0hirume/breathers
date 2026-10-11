use std::collections::BTreeSet;

use tree_sitter::Node;

pub(super) fn related(previous: Node<'_>, next: Node<'_>, source: &str) -> bool {
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
