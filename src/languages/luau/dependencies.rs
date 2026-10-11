use std::collections::BTreeSet;

use vermis::tree::{NodeIndex, NodeKind, Tree};

use super::statements::{root, text};

pub(super) fn related(previous: NodeIndex, next: NodeIndex, tree: &Tree<'_>, source: &str) -> bool {
    let names: BTreeSet<_> = match &tree.node(previous).kind {
        NodeKind::Local { bindings, .. } | NodeKind::Constant { bindings, .. } => tree
            .list(bindings)
            .iter()
            .filter_map(|binding| {
                if let NodeKind::Binding { name, .. } = tree.node(binding.node).kind {
                    Some(text(name, tree, source))
                } else {
                    None
                }
            })
            .collect(),

        NodeKind::Assignment { targets, .. } => tree
            .list(targets)
            .iter()
            .filter_map(|target| root(target.node, tree, source))
            .collect(),

        NodeKind::CompoundAssignment { target, .. } => {
            root(*target, tree, source).into_iter().collect()
        }

        _ => return false,
    };

    !names.is_empty() && reads(next, tree, source, &names)
}

fn reads(node: NodeIndex, tree: &Tree<'_>, source: &str, names: &BTreeSet<&str>) -> bool {
    let uses = |node| reads(node, tree, source, names);

    match &tree.node(node).kind {
        NodeKind::Name { .. } => names.contains(text(node, tree, source)),

        NodeKind::Local { values, .. }
        | NodeKind::Constant { values, .. }
        | NodeKind::GenericFor { values, .. } => {
            tree.list(values).iter().any(|entry| uses(entry.node))
        }

        NodeKind::Assignment {
            targets, values, ..
        } => {
            tree.list(values).iter().any(|entry| uses(entry.node))
                || tree.list(targets).iter().any(|target| {
                    !matches!(tree.node(target.node).kind, NodeKind::Name { .. })
                        && uses(target.node)
                })
        }

        NodeKind::If { branches, .. } => tree
            .list(branches)
            .first()
            .is_some_and(|entry| uses(entry.node)),

        NodeKind::Branch { condition, .. }
        | NodeKind::While { condition, .. }
        | NodeKind::Repeat { condition, .. } => uses(*condition),

        NodeKind::NumericFor {
            start, end, step, ..
        } => uses(*start) || uses(*end) || step.is_some_and(uses),

        NodeKind::MethodCall {
            receiver,
            arguments,
            ..
        } => uses(*receiver) || uses(*arguments),

        NodeKind::Field { receiver, .. } => uses(*receiver),

        NodeKind::Group { expression, .. }
        | NodeKind::Assertion { expression, .. }
        | NodeKind::Instantiate { expression, .. } => uses(*expression),

        NodeKind::TableField {
            key,
            value,
            opening,
            ..
        } => uses(*value) || (opening.is_some() && key.is_some_and(uses)),

        NodeKind::Return { .. }
        | NodeKind::Arguments { .. }
        | NodeKind::CompoundAssignment { .. }
        | NodeKind::CallStatement { .. }
        | NodeKind::Call { .. }
        | NodeKind::Index { .. }
        | NodeKind::Unary { .. }
        | NodeKind::Binary { .. }
        | NodeKind::Conditional { .. }
        | NodeKind::Table { .. }
        | NodeKind::Interpolation { .. } => tree.children(node).into_iter().any(uses),

        _ => false,
    }
}
