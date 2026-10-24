use std::collections::BTreeSet;

use miette::LabeledSpan;
use vermis::{
    token::TokenKind,
    tree::{NodeIndex, NodeKind, Tree},
};

use crate::{
    configuration::{Rule, Rules},
    error::Failure,
    spacing,
};

struct Statement<'source> {
    node: NodeIndex,
    complex: bool,
    declaration: Option<&'source str>,
    assignment: Option<&'source str>,
    group: Option<Group<'source>>,
}

impl<'source> Statement<'source> {
    fn new(node: NodeIndex, tree: &Tree<'_>, source: &'source str, rules: &Rules) -> Self {
        let complex = complex(node, tree, source, rules);
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

    fn separates(
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

pub(crate) fn breathe(source: &str, rules: &Rules) -> Result<String, Failure> {
    let tree = vermis::parse(source.as_bytes());

    if !tree.diagnostics.is_empty() {
        return Err(Failure::syntax(
            source,
            tree.diagnostics
                .iter()
                .map(|diagnostic| {
                    LabeledSpan::new_with_span(
                        Some(diagnostic.message.to_string()),
                        diagnostic.span.start..diagnostic.span.end,
                    )
                })
                .collect(),
        ));
    }

    let top = &tree.node(tree.root).kind;
    let mut insertions = BTreeSet::new();

    for (index, node) in tree.nodes.iter().enumerate() {
        let NodeKind::Block { statements } = &node.kind else {
            continue;
        };

        let module = matches!(top, NodeKind::Root { block, .. } if block.get() == index);

        let statements: Vec<_> = tree
            .list(statements)
            .iter()
            .map(|entry| Statement::new(entry.node, &tree, source, rules))
            .collect();

        for (position, pair) in statements.windows(2).enumerate() {
            let previous = &pair[0];
            let next = &pair[1];

            let ordinary = previous.complex
                || next.complex
                || (rules.enabled(Rule::ReturnStatements)
                    && matches!(tree.node(next.node).kind, NodeKind::Return { .. }));

            let grouping = previous.separates(next, statements.get(position + 2), module, rules);

            if !(ordinary || grouping)
                || (rules.enabled(Rule::Related)
                    && related(previous.node, next.node, &tree, source))
            {
                continue;
            }

            let start = tree.node(previous.node).span.end;
            let end = tree.node(next.node).span.start;

            let first = tree
                .tokens
                .partition_point(|token| token.span.start < start);

            let comments = tree.tokens[first..]
                .iter()
                .take_while(|token| token.span.end <= end)
                .filter(|token| matches!(token.kind, TokenKind::Comment | TokenKind::BlockComment))
                .map(|token| token.span.start..token.span.end);

            if let Some(boundary) = spacing::boundary(source, start, end, comments) {
                insertions.insert(boundary);
            }
        }
    }

    Ok(spacing::apply(source, insertions))
}

fn text<'source>(node: NodeIndex, tree: &Tree<'_>, source: &'source str) -> &'source str {
    let span = tree.node(node).span;

    &source[span.start..span.end]
}

fn root<'source>(node: NodeIndex, tree: &Tree<'_>, source: &'source str) -> Option<&'source str> {
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

fn complex(node: NodeIndex, tree: &Tree<'_>, source: &str, rules: &Rules) -> bool {
    if let Some(rule) = block(node, tree) {
        return rules.enabled(rule);
    }

    let fallback = if matches!(
        tree.node(node).kind,
        NodeKind::Local { .. } | NodeKind::Constant { .. }
    ) {
        Rule::Declarations
    } else {
        Rule::Multiline
    };

    let rule = expression(node, tree).unwrap_or(fallback);

    rules.enabled(rule) && multiline(node, tree, source)
}

fn block(node: NodeIndex, tree: &Tree<'_>) -> Option<Rule> {
    match tree.node(node).kind {
        NodeKind::If { .. } => Some(Rule::Conditionals),
        NodeKind::While { .. } => Some(Rule::WhileLoops),
        NodeKind::Repeat { .. } => Some(Rule::RepeatLoops),
        NodeKind::NumericFor { .. } | NodeKind::GenericFor { .. } => Some(Rule::ForLoops),
        NodeKind::Do { .. } => Some(Rule::DoBlocks),
        NodeKind::Function { body: Some(_), .. } => Some(Rule::Functions),
        NodeKind::Class { .. } => Some(Rule::Classes),
        _ => None,
    }
}

fn expression(node: NodeIndex, tree: &Tree<'_>) -> Option<Rule> {
    match tree.node(node).kind {
        NodeKind::TypeAlias { .. } => Some(Rule::TypeAliases),
        NodeKind::Call { .. } | NodeKind::MethodCall { .. } => Some(Rule::Calls),
        NodeKind::Table { .. } => Some(Rule::Tables),
        NodeKind::Interpolation { .. } | NodeKind::Else { .. } => None,

        _ => tree
            .children(node)
            .into_iter()
            .find_map(|child| expression(child, tree)),
    }
}

fn multiline(node: NodeIndex, tree: &Tree<'_>, source: &str) -> bool {
    let tokens = &tree.node(node).tokens;

    tree.tokens[tokens.start.get()..tokens.end.get()]
        .iter()
        .any(|token| {
            token.kind == TokenKind::Whitespace
                && source[token.span.start..token.span.end].contains('\n')
        })
}

fn related(previous: NodeIndex, next: NodeIndex, tree: &Tree<'_>, source: &str) -> bool {
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
