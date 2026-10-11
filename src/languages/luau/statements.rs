use vermis::tree::{NodeIndex, NodeKind, Tree};

use super::classification;
use crate::configuration::{Rule, Rules};

pub(super) struct Statement<'source> {
    pub(super) node: NodeIndex,
    pub(super) complex: bool,
    declaration: Option<&'source str>,
    assignment: Option<&'source str>,
    group: Option<Group<'source>>,
}

impl<'source> Statement<'source> {
    pub(super) fn new(
        node: NodeIndex,
        tree: &Tree<'_>,
        source: &'source str,
        rules: &Rules,
    ) -> Self {
        let complex = classification::complex(node, tree, source, rules);
        let declaration = declaration(node, tree, source);
        let assignment = assignment(node, tree, source);
        let group = group(node, tree, source, declaration, assignment);

        Self {
            node,
            complex,
            declaration,
            assignment,
            group,
        }
    }

    pub(super) fn separates(
        &self,
        next: &Self,
        following: Option<&Self>,
        module: bool,
        rules: &Rules,
    ) -> bool {
        if next.assignment.is_some() && next.assignment == self.declaration.or(self.assignment) {
            return false;
        }

        if module
            && let (Some(left), Some(right)) = (&self.group, &next.group)
            && left != right
            && (rules.enabled(left.rule()) || rules.enabled(right.rule()))
        {
            return true;
        }

        if !rules.enabled(Rule::PopulatedDeclarations) {
            return false;
        }

        let populated = next.declaration.is_some_and(|name| {
            following.is_some_and(|following| following.assignment == Some(name))
        });

        populated
            || (self.assignment.is_some() && next.declaration.is_some())
            || (self.declaration.is_some() && next.assignment.is_some())
    }
}

#[derive(PartialEq)]
enum Group<'source> {
    Service,
    Require(&'source str),
    Imported,
    Local,
    Exported,
    Class,
}

impl Group<'_> {
    fn rule(&self) -> Rule {
        match self {
            Self::Service => Rule::Services,
            Self::Require(_) => Rule::Requires,
            Self::Imported | Self::Local | Self::Exported => Rule::TypeGroups,
            Self::Class => Rule::ClassGroups,
        }
    }
}

pub(super) fn text<'source>(
    node: NodeIndex,
    tree: &Tree<'_>,
    source: &'source str,
) -> &'source str {
    let span = tree.node(node).span;

    &source[span.start..span.end]
}

pub(super) fn root<'source>(
    node: NodeIndex,
    tree: &Tree<'_>,
    source: &'source str,
) -> Option<&'source str> {
    match tree.node(node).kind {
        NodeKind::Field { receiver, .. } | NodeKind::Index { receiver, .. } => {
            root(receiver, tree, source)
        }

        NodeKind::Group { expression, .. } | NodeKind::Assertion { expression, .. } => {
            root(expression, tree, source)
        }

        NodeKind::Name { .. } => Some(text(node, tree, source)),
        _ => None,
    }
}

fn declaration<'source>(
    node: NodeIndex,
    tree: &Tree<'_>,
    source: &'source str,
) -> Option<&'source str> {
    let (NodeKind::Local { bindings, .. } | NodeKind::Constant { bindings, .. }) =
        &tree.node(node).kind
    else {
        return None;
    };

    let [binding] = tree.list(bindings) else {
        return None;
    };

    let NodeKind::Binding { name, .. } = tree.node(binding.node).kind else {
        return None;
    };

    Some(text(name, tree, source))
}

fn assignment<'source>(
    node: NodeIndex,
    tree: &Tree<'_>,
    source: &'source str,
) -> Option<&'source str> {
    if let NodeKind::CompoundAssignment { target, .. } = tree.node(node).kind {
        return matches!(
            tree.node(target).kind,
            NodeKind::Field { .. } | NodeKind::Index { .. }
        )
        .then(|| root(target, tree, source))
        .flatten();
    }

    let NodeKind::Assignment { targets, .. } = &tree.node(node).kind else {
        return None;
    };

    let (first, targets) = tree.list(targets).split_first()?;

    if !matches!(
        tree.node(first.node).kind,
        NodeKind::Field { .. } | NodeKind::Index { .. }
    ) {
        return None;
    }

    let name = root(first.node, tree, source)?;

    targets
        .iter()
        .all(|target| {
            matches!(
                tree.node(target.node).kind,
                NodeKind::Field { .. } | NodeKind::Index { .. }
            ) && root(target.node, tree, source) == Some(name)
        })
        .then_some(name)
}

fn group<'source>(
    node: NodeIndex,
    tree: &Tree<'_>,
    source: &'source str,
    declaration: Option<&'source str>,
    assignment: Option<&'source str>,
) -> Option<Group<'source>> {
    match &tree.node(node).kind {
        NodeKind::Export { declaration, .. }
            if matches!(tree.node(*declaration).kind, NodeKind::TypeAlias { .. }) =>
        {
            Some(Group::Exported)
        }

        NodeKind::TypeAlias { annotation, .. } => Some(
            if matches!(
                tree.node(*annotation).kind,
                NodeKind::TypeName {
                    namespace: Some(_),
                    ..
                }
            ) {
                Group::Imported
            } else {
                Group::Local
            },
        ),

        NodeKind::Local { values, .. } | NodeKind::Constant { values, .. } => {
            let value = tree.list(values).first()?.node;

            if let NodeKind::MethodCall {
                receiver, method, ..
            } = tree.node(value).kind
                && text(receiver, tree, source) == "game"
                && text(method, tree, source) == "GetService"
            {
                return Some(Group::Service);
            }

            if let Some(alias) = require(value, tree, source) {
                return Some(Group::Require(alias));
            }

            if matches!(tree.node(value).kind, NodeKind::Table { .. })
                && declaration.is_some_and(|name| name.starts_with(char::is_uppercase))
            {
                return Some(Group::Class);
            }

            None
        }

        NodeKind::Assignment { .. } | NodeKind::CompoundAssignment { .. }
            if assignment.is_some_and(|name| name.starts_with(char::is_uppercase)) =>
        {
            Some(Group::Class)
        }

        _ => None,
    }
}

fn require<'source>(
    node: NodeIndex,
    tree: &Tree<'_>,
    source: &'source str,
) -> Option<&'source str> {
    match tree.node(node).kind {
        NodeKind::Group { expression, .. } | NodeKind::Assertion { expression, .. } => {
            require(expression, tree, source)
        }

        NodeKind::Call { callee, arguments } if text(callee, tree, source) == "require" => {
            let NodeKind::Arguments { values, .. } = &tree.node(arguments).kind else {
                return None;
            };

            let value = tree.list(values).first()?.node;

            if !matches!(tree.node(value).kind, NodeKind::String { .. }) {
                return None;
            }

            let literal = text(value, tree, source);
            let quote = *literal.as_bytes().first()?;

            if !matches!(quote, b'\'' | b'"') || literal.as_bytes().last() != Some(&quote) {
                return None;
            }

            literal.get(1..literal.len() - 1)?.split('/').next()
        }

        _ => None,
    }
}
