use tree_sitter::Node;

use super::dependencies;

use crate::{
    configuration::{Rule, Rules},
    syntax::{self, Pair, Policy, multiline, rule},
};

pub(crate) fn breathe(source: &str, rules: &Rules) -> Result<String, String> {
    let policy = Policy {
        opaque,
        container,
        member,
        separate,
        trivia: |_| true,
        continuation: None,
    };

    syntax::breathe(
        source,
        &tree_sitter_nu::LANGUAGE.into(),
        rules,
        &policy,
        &[],
    )
}

fn container(node: Node<'_>) -> bool {
    matches!(
        node.kind(),
        "nu_script" | "block" | "val_closure" | "expr_parenthesized" | "record_body" | "ctrl_match"
    )
}

fn member(node: Node<'_>, parent: Node<'_>) -> bool {
    !matches!(
        node.kind(),
        "comment" | "shebang" | "parameter_pipes" | "cell_path"
    ) && (parent.kind() != "ctrl_match" || matches!(node.kind(), "match_arm" | "default_arm"))
}

fn separate(pair: Pair<'_>, source: &str, rules: &Rules) -> bool {
    let Pair {
        parent,
        left,
        right,
        ..
    } = pair;

    let separate = if matches!(parent.kind(), "record_body" | "ctrl_match") {
        let rule = if parent.kind() == "record_body" {
            Rule::RecordFields
        } else {
            Rule::MatchArms
        };

        rules.enabled(rule) && (multiline(left, source, opaque) || multiline(right, source, opaque))
    } else {
        complex(left, source, rules)
            || complex(right, source, rules)
            || (rules.enabled(Rule::ReturnStatements) && returns(right, source))
    };

    separate && !(rules.enabled(Rule::Related) && dependencies::related(left, right, source))
}

fn opaque(node: Node<'_>) -> bool {
    matches!(
        node.kind(),
        "comment" | "shebang" | "val_string" | "val_interpolated"
    )
}

fn statement(mut node: Node<'_>) -> Node<'_> {
    while matches!(node.kind(), "pipeline" | "pipe_element") {
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

fn block(node: Node<'_>) -> Option<Rule> {
    let node = statement(node);

    match node.kind() {
        "stmt_let" | "stmt_mut" | "stmt_const" => node.child_by_field_name("value").and_then(block),
        "ctrl_if" => Some(Rule::Conditionals),
        "ctrl_for" => Some(Rule::ForLoops),
        "ctrl_while" => Some(Rule::WhileLoops),
        "ctrl_loop" => Some(Rule::LoopStatements),
        "ctrl_match" => Some(Rule::Matches),
        "ctrl_try" => Some(Rule::TryBlocks),
        "decl_def" | "decl_extern" => Some(Rule::Functions),
        "decl_module" | "decl_export" => Some(Rule::Modules),
        _ => None,
    }
}

fn expression(node: Node<'_>) -> Option<Rule> {
    match node.kind() {
        "pipeline" => {
            let mut cursor = node.walk();

            (node
                .named_children(&mut cursor)
                .filter(|child| child.kind() == "pipe_element")
                .count()
                > 1)
            .then_some(Rule::Pipelines)
        }

        "command" | "where_command" => Some(Rule::Calls),
        "val_list" => Some(Rule::Arrays),
        "val_table" => Some(Rule::Tables),
        "val_record" => Some(Rule::Objects),
        _ => None,
    }
}

fn complex(node: Node<'_>, source: &str, rules: &Rules) -> bool {
    if let Some(rule) = block(node) {
        rules.enabled(rule)
    } else {
        let rule = rule(node, expression, opaque).unwrap_or(
            if matches!(
                node.kind(),
                "stmt_let" | "stmt_mut" | "stmt_const" | "decl_alias"
            ) {
                Rule::Declarations
            } else {
                Rule::Multiline
            },
        );

        rules.enabled(rule) && multiline(node, source, opaque)
    }
}

fn returns(node: Node<'_>, source: &str) -> bool {
    let node = statement(node);

    node.kind() == "command"
        && node
            .child_by_field_name("head")
            .is_some_and(|head| &source[node.start_byte()..head.end_byte()] == "return")
}
