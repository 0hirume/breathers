use std::ops::Range;

use tree_sitter::Node;

use super::dependencies;

use crate::{
    configuration::{Rule, Rules},
    syntax::{self, Pair, Policy, multiline, rule},
};

pub(crate) fn breathe(
    source: &str,
    rules: &Rules,
    protected: &[Range<usize>],
) -> Result<String, String> {
    let policy = Policy {
        opaque: terminal,
        container,
        member,
        separate,
        trivia: |_| true,
        continuation: None,
    };

    syntax::breathe(
        source,
        &tree_sitter_cpp::LANGUAGE.into(),
        rules,
        &policy,
        protected,
    )
}

fn terminal(node: Node<'_>) -> bool {
    opaque(node) || node.kind() == "call_expression"
}

fn container(node: Node<'_>) -> bool {
    matches!(
        node.kind(),
        "compound_statement"
            | "case_statement"
            | "field_declaration_list"
            | "enumerator_list"
            | "translation_unit"
            | "declaration_list"
    )
}

fn member(node: Node<'_>, parent: Node<'_>) -> bool {
    node.kind() != "comment" && Some(node) != parent.child_by_field_name("value")
}

fn separate(pair: Pair<'_>, source: &str, rules: &Rules) -> bool {
    let Pair {
        parent,
        left,
        right,
        ..
    } = pair;

    if left.kind().starts_with("preproc_")
        || right.kind().starts_with("preproc_")
        || left.kind() == "access_specifier"
        || right.kind() == "access_specifier"
    {
        return false;
    }

    let separate = if matches!(parent.kind(), "field_declaration_list" | "enumerator_list") {
        let rule = if parent.kind() == "enumerator_list" {
            Rule::EnumMembers
        } else if parent
            .parent()
            .is_some_and(|parent| parent.kind() == "class_specifier")
        {
            Rule::ClassMembers
        } else {
            Rule::StructFields
        };

        rules.enabled(rule) && (multiline(left, source, opaque) || multiline(right, source, opaque))
    } else if left.kind() == "case_statement" && right.kind() == "case_statement" {
        rules.enabled(Rule::SwitchCases)
            && (multiline(left, source, opaque) || multiline(right, source, opaque))
    } else {
        complex(left, source, rules)
            || complex(right, source, rules)
            || match right.kind() {
                "return_statement" => rules.enabled(Rule::ReturnStatements),
                "co_return_statement" => rules.enabled(Rule::CoroutineReturns),
                _ => false,
            }
    };

    separate && !(rules.enabled(Rule::Related) && dependencies::related(left, right, source))
}

fn opaque(node: Node<'_>) -> bool {
    matches!(
        node.kind(),
        "comment" | "string_literal" | "raw_string_literal" | "char_literal"
    ) || (node.kind().starts_with("preproc_")
        && !matches!(
            node.kind(),
            "preproc_if" | "preproc_ifdef" | "preproc_else" | "preproc_elif" | "preproc_elifdef"
        ))
}

fn block(node: Node<'_>) -> Option<Rule> {
    match node.kind() {
        "if_statement" => Some(Rule::Conditionals),
        "switch_statement" => Some(Rule::Switches),
        "for_statement" | "for_range_loop" => Some(Rule::ForLoops),
        "while_statement" => Some(Rule::WhileLoops),
        "do_statement" | "compound_statement" => Some(Rule::DoBlocks),
        "try_statement" => Some(Rule::TryBlocks),
        "function_definition" => Some(Rule::Functions),
        _ => None,
    }
}

fn expression(node: Node<'_>) -> Option<Rule> {
    match node.kind() {
        "call_expression" => Some(Rule::Calls),
        "initializer_list" => Some(Rule::Arrays),
        _ => None,
    }
}

fn complex(node: Node<'_>, source: &str, rules: &Rules) -> bool {
    if let Some(rule) = block(node) {
        rules.enabled(rule)
    } else {
        let rule = rule(node, expression, opaque).unwrap_or(if node.kind() == "declaration" {
            Rule::Declarations
        } else {
            Rule::Multiline
        });

        rules.enabled(rule) && multiline(node, source, opaque)
    }
}
