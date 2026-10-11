use std::{collections::BTreeSet, ops::Range};

use tree_sitter::{Language, Node, Parser, Tree};

use crate::{
    configuration::{Rule, Rules},
    spacing,
};

pub(crate) struct Policy {
    pub(crate) opaque: fn(Node<'_>) -> bool,
    pub(crate) container: fn(Node<'_>) -> bool,
    pub(crate) member: fn(Node<'_>, Node<'_>) -> bool,
    pub(crate) separate: fn(Pair<'_>, &str, &Rules) -> bool,
    pub(crate) trivia: fn(Node<'_>) -> bool,
    pub(crate) continuation: Option<char>,
}

#[derive(Copy, Clone)]
pub(crate) struct Pair<'tree> {
    pub(crate) parent: Node<'tree>,
    pub(crate) left: Node<'tree>,
    pub(crate) right: Node<'tree>,
    pub(crate) initial: bool,
}

pub(crate) fn breathe(
    source: &str,
    language: &Language,
    rules: &Rules,
    policy: &Policy,
    protected: &[Range<usize>],
) -> Result<String, String> {
    let mut parser = Parser::new();

    parser
        .set_language(language)
        .map_err(|error| error.to_string())?;

    let tree = parser.parse(source, None).ok_or("Could not parse source")?;

    format(source, &tree, rules, policy, protected)
}

pub(crate) fn format(
    source: &str,
    tree: &Tree,
    rules: &Rules,
    policy: &Policy,
    protected: &[Range<usize>],
) -> Result<String, String> {
    if tree.root_node().has_error() {
        return Err("Syntax errors; no changes written".into());
    }

    let mut insertions = BTreeSet::new();

    visit(
        tree.root_node(),
        source,
        rules,
        policy,
        protected,
        &mut insertions,
    );

    Ok(spacing::apply(source, insertions))
}

pub(crate) fn rule(
    node: Node<'_>,
    classify: fn(Node<'_>) -> Option<Rule>,
    opaque: fn(Node<'_>) -> bool,
) -> Option<Rule> {
    if opaque(node) {
        return None;
    }

    if let Some(rule) = classify(node) {
        return Some(rule);
    }

    let mut cursor = node.walk();

    node.named_children(&mut cursor)
        .find_map(|child| rule(child, classify, opaque))
}

pub(crate) fn multiline(node: Node<'_>, source: &str, opaque: fn(Node<'_>) -> bool) -> bool {
    if opaque(node) {
        return false;
    }

    let mut cursor = node.walk();
    let mut previous = node.start_byte();

    for child in node.children(&mut cursor) {
        if source[previous..child.start_byte()].contains('\n') || multiline(child, source, opaque) {
            return true;
        }

        previous = child.end_byte();
    }

    node.child_count() > 0 && source[previous..node.end_byte()].contains('\n')
}

pub(crate) fn statement<'tree>(mut node: Node<'tree>, wrappers: &[&str]) -> Node<'tree> {
    while wrappers.contains(&node.kind()) {
        let mut cursor = node.walk();

        let mut children = node
            .named_children(&mut cursor)
            .filter(|child| child.kind() != "comment");

        let Some(child) = children.next() else { break };

        if children.next().is_some() {
            break;
        }

        node = child;
    }

    node
}

fn visit(
    node: Node<'_>,
    source: &str,
    rules: &Rules,
    policy: &Policy,
    protected: &[Range<usize>],
    insertions: &mut BTreeSet<usize>,
) {
    if (policy.opaque)(node)
        || protected
            .iter()
            .any(|range| range.start <= node.start_byte() && node.end_byte() <= range.end)
    {
        return;
    }

    let mut cursor = node.walk();
    let children: Vec<_> = node.named_children(&mut cursor).collect();

    if (policy.container)(node) {
        let mut previous = None;
        let mut initial = true;

        for (index, child) in children.iter().copied().enumerate() {
            if !(policy.member)(child, node) {
                continue;
            }

            if let Some(previous) = previous {
                let left: Node<'_> = children[previous];

                let pair = Pair {
                    parent: node,
                    left,
                    right: child,
                    initial,
                };

                if (policy.separate)(pair, source, rules)
                    && let Some(offset) = spacing::boundary(
                        source,
                        left.end_byte(),
                        child.start_byte(),
                        children[previous + 1..index]
                            .iter()
                            .copied()
                            .filter(|node| (policy.trivia)(*node))
                            .map(|node| node.byte_range()),
                    )
                    && !protected.iter().any(|range| range.contains(&offset))
                    && !policy.continuation.is_some_and(|character| {
                        source[..offset]
                            .trim_end_matches(['\r', '\n'])
                            .ends_with(character)
                    })
                {
                    insertions.insert(offset);
                }

                initial = false;
            }

            previous = Some(index);
        }
    }

    for child in children {
        visit(child, source, rules, policy, protected, insertions);
    }
}
