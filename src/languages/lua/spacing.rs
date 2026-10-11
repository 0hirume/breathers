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
        &tree_sitter_lua::LANGUAGE.into(),
        rules,
        &policy,
        &[],
    )
}

fn container(node: Node<'_>) -> bool {
    matches!(node.kind(), "chunk" | "block")
}

fn member(node: Node<'_>, _: Node<'_>) -> bool {
    !matches!(node.kind(), "comment" | "empty_statement")
}

fn separate(pair: Pair<'_>, source: &str, rules: &Rules) -> bool {
    let Pair { left, right, .. } = pair;

    left.kind() != "hash_bang_line"
        && (complex(left, source, rules)
            || complex(right, source, rules)
            || (rules.enabled(Rule::ReturnStatements) && right.kind() == "return_statement"))
        && !(rules.enabled(Rule::Related) && dependencies::related(left, right, source))
}

fn opaque(node: Node<'_>) -> bool {
    matches!(node.kind(), "string" | "comment" | "hash_bang_line")
}

fn block(node: Node<'_>) -> Option<Rule> {
    match node.kind() {
        "if_statement" => Some(Rule::Conditionals),
        "while_statement" => Some(Rule::WhileLoops),
        "repeat_statement" => Some(Rule::RepeatLoops),
        "for_statement" => Some(Rule::ForLoops),
        "do_statement" => Some(Rule::DoBlocks),
        "function_declaration" | "function_definition" => Some(Rule::Functions),
        _ => None,
    }
}

fn expression(node: Node<'_>) -> Option<Rule> {
    match node.kind() {
        "function_call" => Some(Rule::Calls),
        "table_constructor" => Some(Rule::Tables),
        _ => None,
    }
}

fn complex(node: Node<'_>, source: &str, rules: &Rules) -> bool {
    if let Some(rule) = block(node) {
        rules.enabled(rule)
    } else {
        let rule =
            rule(node, expression, opaque).unwrap_or(if node.kind() == "variable_declaration" {
                Rule::Declarations
            } else {
                Rule::Multiline
            });

        rules.enabled(rule) && multiline(node, source, opaque)
    }
}
