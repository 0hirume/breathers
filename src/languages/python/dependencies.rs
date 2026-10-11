use std::collections::BTreeSet;

use tree_sitter::Node;

pub(super) fn related(previous: Node<'_>, next: Node<'_>, source: &str) -> bool {
    let mut names = BTreeSet::new();
    written(previous, source, &mut names);

    !names.is_empty() && reads(next, source, &names)
}

fn written<'source>(node: Node<'_>, source: &'source str, names: &mut BTreeSet<&'source str>) {
    if matches!(node.kind(), "assignment" | "augmented_assignment") {
        if let Some(left) = node.child_by_field_name("left") {
            bindings(left, source, names);

            if let Some(right) = node.child_by_field_name("right") {
                written(right, source, names);
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

        "attribute" | "subscript" => {
            if let Some(object) = node
                .child_by_field_name("object")
                .or_else(|| node.child_by_field_name("value"))
            {
                bindings(object, source, names);
            }
        }

        "pattern_list"
        | "tuple_pattern"
        | "list_pattern"
        | "tuple"
        | "list"
        | "list_splat_pattern"
        | "dictionary_splat_pattern"
        | "parenthesized_expression" => {
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
        "attribute" | "subscript" => reads(node, source, names),

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
        | "function_definition"
        | "lambda"
        | "class_definition"
        | "decorated_definition"
        | "block"
        | "else_clause"
        | "elif_clause"
        | "finally_clause"
        | "type_alias_statement"
        | "import_statement"
        | "import_from_statement" => false,

        "string" => {
            let mut cursor = node.walk();

            node.named_children(&mut cursor)
                .any(|child| child.kind() == "interpolation" && reads(child, source, names))
        }

        "list_comprehension"
        | "set_comprehension"
        | "dictionary_comprehension"
        | "generator_expression" => comprehension_reads(node, source, names),

        "attribute" => field("object"),
        "keyword_argument" => field("value"),

        "assignment" => {
            field("right")
                || node
                    .child_by_field_name("left")
                    .is_some_and(|left| target_reads(left, source, names))
                || (node
                    .child_by_field_name("operator")
                    .is_some_and(|operator| &source[operator.byte_range()] != "=")
                    && field("left"))
        }

        "augmented_assignment" => field("right") || field("left"),
        "for_statement" if node.child_by_field_name("right").is_some() => field("right"),

        _ => {
            let mut cursor = node.walk();

            node.named_children(&mut cursor).any(|child| {
                !["type", "return_type", "alias"]
                    .into_iter()
                    .any(|field| node.child_by_field_name(field) == Some(child))
                    && reads(child, source, names)
            })
        }
    }
}

fn comprehension_reads(node: Node<'_>, source: &str, names: &BTreeSet<&str>) -> bool {
    let mut visible = names.clone();
    let mut cursor = node.walk();

    for clause in node.named_children(&mut cursor) {
        if clause.kind() == "for_in_clause" {
            let mut cursor = clause.walk();

            if clause
                .children_by_field_name("right", &mut cursor)
                .any(|value| reads(value, source, &visible))
            {
                return true;
            }

            if let Some(left) = clause.child_by_field_name("left") {
                let mut shadowed = BTreeSet::new();
                bindings(left, source, &mut shadowed);
                visible.retain(|name| !shadowed.contains(name));
            }
        } else if clause.kind() == "if_clause" && reads(clause, source, &visible) {
            return true;
        }
    }

    node.child_by_field_name("body")
        .is_some_and(|body| reads(body, source, &visible))
}
