use std::collections::BTreeSet;

use vermis::{token::TokenKind, tree::NodeKind};

use super::{dependencies, statements::Statement};

use crate::{
    configuration::{Rule, Rules},
    spacing,
};

pub(crate) fn breathe(source: &str, rules: &Rules) -> Result<String, String> {
    let tree = vermis::parse(source.as_bytes());

    if !tree.diagnostics.is_empty() {
        return Err(tree
            .diagnostics
            .iter()
            .map(|diagnostic| format!("{} at byte {}", diagnostic.message, diagnostic.span.start))
            .collect::<Vec<_>>()
            .join("; "));
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
                    && dependencies::related(previous.node, next.node, &tree, source))
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
