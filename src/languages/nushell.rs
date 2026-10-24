use std::collections::BTreeSet;

use tree_sitter::Node;

use crate::{
    configuration::{Rule, Rules},
    error::Failure,
    syntax::{self, Pair, Policy, multiline, rule},
};

const WRAPPERS: &[&str] = &["pipeline", "pipe_element"];

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

        rules.enabled_or(rule, false)
            && (multiline(left, source, opaque) || multiline(right, source, opaque))
    } else {
        complex(left, source, rules)
            || complex(right, source, rules)
            || (rules.enabled(Rule::ReturnStatements) && returns(right, source))
    };

    separate && !(rules.enabled(Rule::Related) && related(left, right, source))
}

fn opaque(node: Node<'_>) -> bool {
    matches!(
        node.kind(),
        "comment" | "shebang" | "val_string" | "val_interpolated"
    )
}

fn block(node: Node<'_>) -> Option<Rule> {
    let node = syntax::statement(node, WRAPPERS);

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
    let node = syntax::statement(node, WRAPPERS);

    node.kind() == "command"
        && node
            .child_by_field_name("head")
            .is_some_and(|head| &source[node.start_byte()..head.end_byte()] == "return")
}

fn related(previous: Node<'_>, next: Node<'_>, source: &str) -> bool {
    let mut names = BTreeSet::new();
    written(previous, source, &mut names);

    !names.is_empty() && reads(next, source, &names)
}

fn written<'source>(node: Node<'_>, source: &'source str, names: &mut BTreeSet<&'source str>) {
    let field = match node.kind() {
        "stmt_let" | "stmt_mut" | "stmt_const" => Some("name"),
        "assignment" => Some("lhs"),
        _ => None,
    };

    if let Some(target) = field.and_then(|field| node.child_by_field_name(field)) {
        bindings(target, source, names);
    } else if matches!(node.kind(), "pipeline" | "pipe_element") && node.named_child_count() == 1 {
        written(node.named_child(0).unwrap(), source, names);
    }
}

fn bindings<'source>(node: Node<'_>, source: &'source str, names: &mut BTreeSet<&'source str>) {
    match node.kind() {
        "identifier" => {
            names.insert(&source[node.byte_range()]);
        }

        "val_variable" => {
            if let Some(name) = node.child_by_field_name("name") {
                bindings(name, source, names);
            }
        }

        _ => {}
    }
}

fn target_reads(node: Node<'_>, source: &str, names: &BTreeSet<&str>) -> bool {
    match node.kind() {
        "identifier" => false,
        "val_variable" => node.named_child_count() > 1 && reads(node, source, names),

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

        "val_variable" => node
            .child_by_field_name("name")
            .is_some_and(|name| names.contains(&source[name.byte_range()])),

        "comment" | "block" | "val_string" | "val_closure" | "decl_def" | "decl_extern"
        | "decl_module" | "decl_export" | "decl_alias" | "decl_use" | "cell_path" => false,

        "ctrl_if" | "ctrl_while" => field("condition"),
        "ctrl_for" => field("iterable"),
        "ctrl_match" => field("scrutinee"),

        "record_entry" => {
            field("value")
                || node
                    .child_by_field_name("key")
                    .is_some_and(|key| key.kind() != "identifier" && reads(key, source, names))
        }

        "stmt_let" | "stmt_mut" | "stmt_const" | "env_var" => field("value"),

        "assignment" => {
            field("rhs")
                || node
                    .child_by_field_name("lhs")
                    .is_some_and(|left| target_reads(left, source, names))
                || (node
                    .child_by_field_name("opr")
                    .is_some_and(|operator| &source[operator.byte_range()] != "=")
                    && field("lhs"))
        }

        _ => {
            let mut cursor = node.walk();

            node.named_children(&mut cursor).any(|child| {
                !["type", "return_type"]
                    .into_iter()
                    .any(|field| node.child_by_field_name(field) == Some(child))
                    && reads(child, source, names)
            })
        }
    }
}
