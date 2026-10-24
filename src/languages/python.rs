use std::collections::BTreeSet;

use tree_sitter::Node;

use crate::{
    configuration::{Rule, Rules},
    error::Failure,
    syntax::{self, Pair, Policy, multiline, rule},
};

pub(crate) fn breathe(source: &str, rules: &Rules) -> Result<String, Failure> {
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
        rules.enabled_or(Rule::DictionaryEntries, false)
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
        && !(rules.enabled(Rule::Related) && related(left, right, source))
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

fn related(previous: Node<'_>, next: Node<'_>, source: &str) -> bool {
    let mut names = BTreeSet::new();
    written(previous, source, &mut names);

    !names.is_empty() && reads(next, source, &names)
}

fn written<'source>(node: Node<'_>, source: &'source str, names: &mut BTreeSet<&'source str>) {
    if matches!(node.kind(), "assignment" | "augmented_assignment") {
        if let Some(left) = node.child_by_field_name("left") {
            bindings(left, source, names);

            if let Some(right) = node.child_by_field_name("right") {
                written(right, source, names);
            }
        }
    } else if node.kind() == "expression_statement" {
        let mut cursor = node.walk();

        for child in node.named_children(&mut cursor) {
            written(child, source, names);
        }
    }
}

fn bindings<'source>(node: Node<'_>, source: &'source str, names: &mut BTreeSet<&'source str>) {
    match node.kind() {
        "identifier" => {
            names.insert(&source[node.byte_range()]);
        }

        "attribute" | "subscript" => {
            if let Some(object) = node
                .child_by_field_name("object")
                .or_else(|| node.child_by_field_name("value"))
            {
                bindings(object, source, names);
            }
        }

        "pattern_list"
        | "tuple_pattern"
        | "list_pattern"
        | "tuple"
        | "list"
        | "list_splat_pattern"
        | "dictionary_splat_pattern"
        | "parenthesized_expression" => {
            let mut cursor = node.walk();

            for child in node.named_children(&mut cursor) {
                bindings(child, source, names);
            }
        }

        _ => {}
    }
}

fn target_reads(node: Node<'_>, source: &str, names: &BTreeSet<&str>) -> bool {
    match node.kind() {
        "identifier" => false,
        "attribute" | "subscript" => reads(node, source, names),

        _ => {
            let mut cursor = node.walk();

            node.named_children(&mut cursor)
                .any(|child| target_reads(child, source, names))
        }
    }
}

fn reads(node: Node<'_>, source: &str, names: &BTreeSet<&str>) -> bool {
    let field = |field| {
        node.child_by_field_name(field)
            .is_some_and(|child| reads(child, source, names))
    };

    match node.kind() {
        "identifier" => names.contains(&source[node.byte_range()]),

        "comment"
        | "function_definition"
        | "lambda"
        | "class_definition"
        | "decorated_definition"
        | "block"
        | "else_clause"
        | "elif_clause"
        | "finally_clause"
        | "type_alias_statement"
        | "import_statement"
        | "import_from_statement" => false,

        "string" => {
            let mut cursor = node.walk();

            node.named_children(&mut cursor)
                .any(|child| child.kind() == "interpolation" && reads(child, source, names))
        }

        "list_comprehension"
        | "set_comprehension"
        | "dictionary_comprehension"
        | "generator_expression" => comprehension_reads(node, source, names),

        "attribute" => field("object"),
        "keyword_argument" => field("value"),

        "assignment" => {
            field("right")
                || node
                    .child_by_field_name("left")
                    .is_some_and(|left| target_reads(left, source, names))
                || (node
                    .child_by_field_name("operator")
                    .is_some_and(|operator| &source[operator.byte_range()] != "=")
                    && field("left"))
        }

        "augmented_assignment" => field("right") || field("left"),
        "for_statement" if node.child_by_field_name("right").is_some() => field("right"),

        _ => {
            let mut cursor = node.walk();

            node.named_children(&mut cursor).any(|child| {
                !["type", "return_type", "alias"]
                    .into_iter()
                    .any(|field| node.child_by_field_name(field) == Some(child))
                    && reads(child, source, names)
            })
        }
    }
}

fn comprehension_reads(node: Node<'_>, source: &str, names: &BTreeSet<&str>) -> bool {
    let mut visible = names.clone();
    let mut cursor = node.walk();

    for clause in node.named_children(&mut cursor) {
        if clause.kind() == "for_in_clause" {
            let mut cursor = clause.walk();

            if clause
                .children_by_field_name("right", &mut cursor)
                .any(|value| reads(value, source, &visible))
            {
                return true;
            }

            if let Some(left) = clause.child_by_field_name("left") {
                let mut shadowed = BTreeSet::new();
                bindings(left, source, &mut shadowed);
                visible.retain(|name| !shadowed.contains(name));
            }
        } else if clause.kind() == "if_clause" && reads(clause, source, &visible) {
            return true;
        }
    }

    node.child_by_field_name("body")
        .is_some_and(|body| reads(body, source, &visible))
}
