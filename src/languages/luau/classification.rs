use vermis::{
    token::TokenKind,
    tree::{NodeIndex, NodeKind, Tree},
};

use crate::configuration::{Rule, Rules};

pub(super) fn complex(node: NodeIndex, tree: &Tree<'_>, source: &str, rules: &Rules) -> bool {
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
