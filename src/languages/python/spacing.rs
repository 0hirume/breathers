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
        &tree_sitter_python::LANGUAGE.into(),
        rules,
        &policy,
        &[],
    )
}

fn container(node: Node<'_>) -> bool {
    matches!(node.kind(), "module" | "block" | "dictionary")
}

fn member(node: Node<'_>, _: Node<'_>) -> bool {
    node.kind() != "comment"
}

fn separate(pair: Pair<'_>, source: &str, rules: &Rules) -> bool {
    let Pair {
        parent,
        left,
        right,
        initial,
    } = pair;

    let separate = if parent.kind() == "dictionary" {
        rules.enabled(Rule::DictionaryEntries)
            && (multiline(left, source, opaque) || multiline(right, source, opaque))
    } else if left.kind() == "case_clause" && right.kind() == "case_clause" {
        rules.enabled(Rule::MatchCases)
            && (multiline(left, source, opaque) || multiline(right, source, opaque))
    } else {
        complex(left, source, rules)
            || complex(right, source, rules)
            || (rules.enabled(Rule::ReturnStatements) && right.kind() == "return_statement")
    };

    separate
        && ![left, right].into_iter().all(|node| {
            matches!(
                node.kind(),
                "import_statement" | "import_from_statement" | "future_import_statement"
            )
        })
        && !(parent.kind() == "block"
            && parent
                .parent()
                .is_some_and(|parent| parent.kind() == "function_definition")
            && initial
            && docstring(left, source))
        && !(rules.enabled(Rule::Related) && dependencies::related(left, right, source))
}

fn opaque(node: Node<'_>) -> bool {
    matches!(node.kind(), "comment" | "string" | "concatenated_string")
}

fn docstring(node: Node<'_>, source: &str) -> bool {
    match node.kind() {
        "expression_statement" => node
            .named_child(0)
            .is_some_and(|child| docstring(child, source)),

        "concatenated_string" => {
            let mut cursor = node.walk();

            node.named_children(&mut cursor)
                .all(|child| docstring(child, source))
        }

        "string" => node.named_child(0).is_some_and(|start| {
            source[start.byte_range()]
                .bytes()
                .all(|byte| matches!(byte, b'\'' | b'"' | b'r' | b'R' | b'u' | b'U'))
        }),

        _ => false,
    }
}

fn block(node: Node<'_>) -> Option<Rule> {
    match node.kind() {
        "if_statement" => Some(Rule::Conditionals),
        "for_statement" => Some(Rule::ForLoops),
        "while_statement" => Some(Rule::WhileLoops),
        "with_statement" => Some(Rule::WithBlocks),
        "try_statement" => Some(Rule::TryBlocks),
        "match_statement" => Some(Rule::Matches),
        "function_definition" => Some(Rule::Functions),
        "class_definition" => Some(Rule::Classes),

        "decorated_definition" => Some(
            if node
                .child_by_field_name("definition")
                .is_some_and(|definition| definition.kind() == "class_definition")
            {
                Rule::Classes
            } else {
                Rule::DecoratedFunctions
            },
        ),

        _ => None,
    }
}

fn expression(node: Node<'_>) -> Option<Rule> {
    match node.kind() {
        "call" => Some(Rule::Calls),

        "list" | "tuple" | "set" | "list_comprehension" | "set_comprehension" => Some(Rule::Arrays),

        "dictionary" | "dictionary_comprehension" => Some(Rule::Objects),
        _ => None,
    }
}

fn complex(node: Node<'_>, source: &str, rules: &Rules) -> bool {
    if let Some(rule) = block(node) {
        rules.enabled(rule)
    } else {
        let rule = if node.kind() == "type_alias_statement" {
            Rule::TypeAliases
        } else {
            rule(node, expression, opaque).unwrap_or(
                if node.kind() == "expression_statement"
                    && node
                        .named_child(0)
                        .is_some_and(|child| child.kind() == "assignment")
                {
                    Rule::Declarations
                } else {
                    Rule::Multiline
                },
            )
        };

        rules.enabled(rule) && multiline(node, source, opaque)
    }
}
