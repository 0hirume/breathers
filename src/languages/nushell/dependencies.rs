use std::collections::BTreeSet;

use tree_sitter::Node;

pub(super) fn related(previous: Node<'_>, next: Node<'_>, source: &str) -> bool {
    let mut names = BTreeSet::new();
    written(previous, source, &mut names);

    !names.is_empty() && reads(next, source, &names)
}

fn written<'source>(node: Node<'_>, source: &'source str, names: &mut BTreeSet<&'source str>) {
    let field = match node.kind() {
        "stmt_let" | "stmt_mut" | "stmt_const" => Some("name"),
        "assignment" => Some("lhs"),
        _ => None,
    };

    if let Some(target) = field.and_then(|field| node.child_by_field_name(field)) {
        bindings(target, source, names);
    } else if matches!(node.kind(), "pipeline" | "pipe_element") && node.named_child_count() == 1 {
        written(node.named_child(0).unwrap(), source, names);
    }
}

fn bindings<'source>(node: Node<'_>, source: &'source str, names: &mut BTreeSet<&'source str>) {
    match node.kind() {
        "identifier" => {
            names.insert(&source[node.byte_range()]);
        }

        "val_variable" => {
            if let Some(name) = node.child_by_field_name("name") {
                bindings(name, source, names);
            }
        }

        _ => {}
    }
}

fn target_reads(node: Node<'_>, source: &str, names: &BTreeSet<&str>) -> bool {
    match node.kind() {
        "identifier" => false,
        "val_variable" => node.named_child_count() > 1 && reads(node, source, names),

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

        "val_variable" => node
            .child_by_field_name("name")
            .is_some_and(|name| names.contains(&source[name.byte_range()])),

        "comment" | "block" | "val_string" | "val_closure" | "decl_def" | "decl_extern"
        | "decl_module" | "decl_export" | "decl_alias" | "decl_use" | "cell_path" => false,

        "ctrl_if" | "ctrl_while" => field("condition"),
        "ctrl_for" => field("iterable"),
        "ctrl_match" => field("scrutinee"),

        "record_entry" => {
            field("value")
                || node
                    .child_by_field_name("key")
                    .is_some_and(|key| key.kind() != "identifier" && reads(key, source, names))
        }

        "stmt_let" | "stmt_mut" | "stmt_const" | "env_var" => field("value"),

        "assignment" => {
            field("rhs")
                || node
                    .child_by_field_name("lhs")
                    .is_some_and(|left| target_reads(left, source, names))
                || (node
                    .child_by_field_name("opr")
                    .is_some_and(|operator| &source[operator.byte_range()] != "=")
                    && field("lhs"))
        }

        _ => {
            let mut cursor = node.walk();

            node.named_children(&mut cursor).any(|child| {
                !["type", "return_type"]
                    .into_iter()
                    .any(|field| node.child_by_field_name(field) == Some(child))
                    && reads(child, source, names)
            })
        }
    }
}
