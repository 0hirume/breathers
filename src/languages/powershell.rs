use std::iter::from_fn;

use tree_sitter::Node;

use crate::{
    configuration::{Rule, Rules},
    error::Failure,
    syntax::{self, Pair, Policy, multiline, rule},
};

const WRAPPERS: &[&str] = &["pipeline", "assignment_value"];

pub(crate) fn breathe(source: &str, rules: &Rules) -> Result<String, Failure> {
    let policy = Policy {
        opaque,
        container,
        member,
        separate,
        trivia: |node| node.kind() == "comment",
        continuation: Some('`'),
    };

    syntax::breathe(
        source,
        &tree_sitter_pwsh::LANGUAGE.into(),
        rules,
        &policy,
        &[],
    )
}

fn container(node: Node<'_>) -> bool {
    matches!(
        node.kind(),
        "statement_list"
            | "named_block_list"
            | "hash_literal_body"
            | "switch_clauses"
            | "class_statement"
            | "enum_statement"
    )
}

fn separate(pair: Pair<'_>, source: &str, rules: &Rules) -> bool {
    let Pair {
        parent,
        left,
        right,
        ..
    } = pair;

    let rule = match parent.kind() {
        "hash_literal_body" => Some(Rule::DictionaryEntries),
        "switch_clauses" => Some(Rule::SwitchCases),
        "class_statement" => Some(Rule::ClassMembers),
        "enum_statement" => Some(Rule::EnumMembers),
        _ => None,
    };

    let separate = if let Some(rule) = rule {
        rules.enabled(rule) && (multiline(left, source, opaque) || multiline(right, source, opaque))
    } else {
        complex(left, source, rules)
            || complex(right, source, rules)
            || (rules.enabled(Rule::ReturnStatements) && returns(right, source))
    };

    separate && !(rule.is_none() && rules.enabled(Rule::Related) && related(left, right, source))
}

fn opaque(node: Node<'_>) -> bool {
    matches!(
        node.kind(),
        "comment"
            | "string_literal"
            | "expandable_string_literal"
            | "expandable_here_string_literal"
            | "verbatim_string_characters"
            | "verbatim_here_string_characters"
    )
}

fn block(node: Node<'_>) -> Option<Rule> {
    let node = syntax::statement(node, WRAPPERS);

    match node.kind() {
        "assignment_expression" => node.child_by_field_name("value").and_then(block),
        "if_statement" => Some(Rule::Conditionals),
        "for_statement" | "foreach_statement" => Some(Rule::ForLoops),
        "while_statement" => Some(Rule::WhileLoops),
        "do_statement" => Some(Rule::DoBlocks),
        "switch_statement" => Some(Rule::Switches),
        "try_statement" | "trap_statement" => Some(Rule::TryBlocks),
        "function_statement" => Some(Rule::Functions),
        "class_statement" | "enum_statement" => Some(Rule::Classes),

        "named_block"
        | "data_statement"
        | "inlinescript_statement"
        | "parallel_statement"
        | "sequence_statement" => Some(Rule::Blocks),

        _ => None,
    }
}

fn expression(node: Node<'_>) -> Option<Rule> {
    match node.kind() {
        "pipeline" | "assignment_value" | "pipeline_chain" => {
            let mut cursor = node.walk();

            node.children(&mut cursor)
                .any(|child| matches!(child.kind(), "|" | "pipeline_chain_tail"))
                .then_some(Rule::Pipelines)
        }

        "command"
        | "invocation_expression"
        | "invocation_foreach_expression"
        | "invocation_where_expression" => Some(Rule::Calls),

        "array_expression" | "array_literal_expression" => Some(Rule::Arrays),
        "hash_literal_expression" => Some(Rule::Objects),
        _ => None,
    }
}

fn complex(node: Node<'_>, source: &str, rules: &Rules) -> bool {
    if let Some(rule) = block(node) {
        rules.enabled(rule)
    } else {
        let fallback = if syntax::statement(node, WRAPPERS).kind() == "assignment_expression" {
            Rule::Declarations
        } else {
            Rule::Multiline
        };

        let rule = rule(node, expression, opaque).unwrap_or(fallback);

        rules.enabled(rule) && multiline(node, source, opaque)
    }
}

fn returns(node: Node<'_>, source: &str) -> bool {
    node.kind() == "flow_control_statement"
        && node
            .child(0)
            .is_some_and(|keyword| source[keyword.byte_range()].eq_ignore_ascii_case("return"))
}

fn member(node: Node<'_>, parent: Node<'_>) -> bool {
    match parent.kind() {
        "class_statement" => matches!(
            node.kind(),
            "class_property_definition" | "class_method_definition"
        ),

        "enum_statement" => node.kind() == "enum_member",
        _ => !matches!(node.kind(), "comment" | "empty_statement" | "label"),
    }
}

fn related(previous: Node<'_>, next: Node<'_>, source: &str) -> bool {
    let previous = syntax::statement(previous, WRAPPERS);

    previous.kind() == "assignment_expression"
        && previous
            .named_child(0)
            .is_some_and(|target| dependencies(target, next, source))
}

fn variable(source: &str) -> impl Iterator<Item = char> + '_ {
    let source = source.trim_start_matches(['$', '@']);

    let source = source
        .strip_prefix('{')
        .and_then(|source| source.strip_suffix('}'))
        .unwrap_or(source);

    let mut characters = source.chars();

    from_fn(move || match characters.next()? {
        '`' => characters.next(),
        character => Some(character),
    })
    .flat_map(char::to_lowercase)
}

fn reads(node: Node<'_>, target: Node<'_>, source: &str) -> bool {
    match node.kind() {
        "variable" => {
            variable(&source[node.byte_range()]).eq(variable(&source[target.byte_range()]))
        }

        "comment"
        | "verbatim_string_characters"
        | "verbatim_here_string_characters"
        | "function_statement"
        | "class_statement"
        | "script_block_expression"
        | "statement_block"
        | "type_literal" => false,

        "assignment_expression" => {
            node.child_by_field_name("value")
                .is_some_and(|value| reads(value, target, source))
                || (node
                    .named_child(1)
                    .is_some_and(|operator| &source[operator.byte_range()] != "=")
                    && node
                        .named_child(0)
                        .is_some_and(|left| reads(left, target, source)))
        }

        _ => {
            let mut cursor = node.walk();

            node.named_children(&mut cursor)
                .any(|child| reads(child, target, source))
        }
    }
}

fn dependencies(target: Node<'_>, next: Node<'_>, source: &str) -> bool {
    if target.kind() == "variable" {
        return reads(next, target, source);
    }

    if matches!(target.kind(), "member_access" | "element_access") {
        return target
            .named_child(0)
            .is_some_and(|receiver| dependencies(receiver, next, source));
    }

    let mut cursor = target.walk();

    target
        .named_children(&mut cursor)
        .any(|child| dependencies(child, next, source))
}
