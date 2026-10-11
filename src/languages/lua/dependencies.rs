use std::collections::BTreeSet;

use tree_sitter::Node;

pub(super) fn related(previous: Node<'_>, next: Node<'_>, source: &str) -> bool {
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
