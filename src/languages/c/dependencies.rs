use std::collections::BTreeSet;

use tree_sitter::Node;

pub(super) fn related(previous: Node<'_>, next: Node<'_>, source: &str) -> bool {
    let mut names = BTreeSet::new();
    written(previous, source, &mut names);

    if let Some(initializer) = next.child_by_field_name("initializer") {
        let mut shadowed = BTreeSet::new();

        if initializer.kind() == "declaration" {
            written(initializer, source, &mut shadowed);
        }

        names.retain(|name| !shadowed.contains(name));
    }

    !names.is_empty() && reads(next, source, &names)
}

fn written<'source>(node: Node<'_>, source: &'source str, names: &mut BTreeSet<&'source str>) {
    let field = match node.kind() {
        "init_declarator" => Some("declarator"),
        "assignment_expression" => Some("left"),
        "update_expression" => Some("argument"),
        _ => None,
    };

    if let Some(target) = field.and_then(|field| node.child_by_field_name(field)) {
        bindings(target, source, names);

        if let Some(right) = node.child_by_field_name("right") {
            written(right, source, names);
        }
    } else if node.kind() == "declaration" {
        let mut cursor = node.walk();

        for declarator in node.children_by_field_name("declarator", &mut cursor) {
            if declarator.kind() == "init_declarator" {
                written(declarator, source, names);
            } else {
                bindings(declarator, source, names);
            }
        }
    } else if node.kind() == "expression_statement" {
        let mut cursor = node.walk();

        for child in node.named_children(&mut cursor) {
            written(child, source, names);
        }
    }
}

fn bindings<'source>(node: Node<'_>, source: &'source str, names: &mut BTreeSet<&'source str>) {
    match node.kind() {
        "identifier" => {
            names.insert(&source[node.byte_range()]);
        }

        "field_expression" | "subscript_expression" | "pointer_expression" => {
            if let Some(argument) = node.child_by_field_name("argument") {
                bindings(argument, source, names);
            }
        }

        "pointer_declarator"
        | "reference_declarator"
        | "array_declarator"
        | "parenthesized_declarator" => {
            if let Some(declarator) = node
                .child_by_field_name("declarator")
                .or_else(|| node.named_child(0))
            {
                bindings(declarator, source, names);
            }
        }

        "structured_binding_declarator" | "parenthesized_expression" => {
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

        "field_expression" | "subscript_expression" | "pointer_expression" => {
            reads(node, source, names)
        }

        _ => {
            let mut cursor = node.walk();

            node.named_children(&mut cursor)
                .any(|child| target_reads(child, source, names))
        }
    }
}

fn reads(node: Node<'_>, source: &str, names: &BTreeSet<&str>) -> bool {
    if node.kind().starts_with("preproc_") {
        return false;
    }

    let field = |field| {
        node.child_by_field_name(field)
            .is_some_and(|child| reads(child, source, names))
    };

    match node.kind() {
        "identifier" => names.contains(&source[node.byte_range()]),

        "string_literal"
        | "raw_string_literal"
        | "char_literal"
        | "comment"
        | "function_definition"
        | "lambda_expression"
        | "compound_statement"
        | "else_clause"
        | "catch_clause"
        | "type_definition"
        | "attribute" => false,

        "field_expression" => field("argument"),
        "init_declarator" => field("value"),

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

        "for_range_loop" => field("right"),

        _ => {
            let mut cursor = node.walk();

            node.named_children(&mut cursor).any(|child| {
                !["type", "label"]
                    .into_iter()
                    .any(|field| node.child_by_field_name(field) == Some(child))
                    && reads(child, source, names)
            })
        }
    }
}
