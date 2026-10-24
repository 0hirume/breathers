use std::{collections::BTreeSet, ops::Range};

use tree_sitter::{Language, Node, Parser};

use crate::{
    configuration::{Rule, Rules},
    error::Failure,
    syntax::{self, Pair, Policy, multiline, rule},
};

pub(crate) fn breathe(source: &str, language: &Language, rules: &Rules) -> Result<String, Failure> {
    let policy = Policy {
        opaque: terminal,
        container,
        member,
        separate,
        trivia: |_| true,
        continuation: None,
    };

    let mut parser = Parser::new();

    parser
        .set_language(language)
        .map_err(|error| error.to_string())?;

    let tree = parser.parse(source, None).ok_or("Could not parse source")?;
    let mut protected = Vec::new();
    let mut prepared = Vec::new();
    prepare(tree.root_node(), source, &mut protected, &mut prepared);

    if prepared.is_empty() {
        return syntax::format(source, &tree, rules, &policy, &protected);
    }

    let tree = parser
        .parse(prepared, None)
        .ok_or("Could not parse source")?;

    syntax::format(source, &tree, rules, &policy, &protected)
}

fn prepare(
    node: Node<'_>,
    source: &str,
    protected: &mut Vec<Range<usize>>,
    prepared: &mut Vec<u8>,
) {
    if let Some((range, initializer)) = region(node, source) {
        if prepared.is_empty() {
            prepared.extend_from_slice(source.as_bytes());
        }

        for byte in &mut prepared[range.clone()] {
            if !matches!(*byte, b'\r' | b'\n') {
                *byte = b' ';
            }
        }

        if initializer {
            prepared[range.start] = b'0';
        }

        protected.push(range);
    }

    let mut cursor = node.walk();

    for child in node.named_children(&mut cursor) {
        if !protected
            .iter()
            .any(|range| range.start <= child.start_byte() && child.end_byte() <= range.end)
        {
            prepare(child, source, protected, prepared);
        }
    }
}

fn region(node: Node<'_>, source: &str) -> Option<(Range<usize>, bool)> {
    if matches!(node.kind(), "preproc_if" | "preproc_elif")
        && let Some(condition) = node.child_by_field_name("condition")
        && source[condition.byte_range()].trim() == "0"
        && let Some(newline) = source[condition.end_byte()..node.end_byte()].find('\n')
    {
        let start = condition.end_byte() + newline + 1;

        let end = node.child_by_field_name("alternative").map_or_else(
            || {
                let mut cursor = node.walk();

                node.children(&mut cursor)
                    .find(|child| child.kind() == "#endif")
                    .map_or(node.end_byte(), |child| child.start_byte())
            },
            |alternative| alternative.start_byte(),
        );

        return Some((start..end, false));
    }

    if invocation(node, source) {
        return Some((node.byte_range(), false));
    }

    if node.kind() == "compound_literal_expression"
        && node
            .child_by_field_name("type")
            .is_some_and(|kind| kind.is_missing())
        && let Some(value) = node.child_by_field_name("value")
        && value.kind() == "initializer_list"
        && !value.has_error()
        && node.parent().is_some_and(|parent| {
            parent.kind() == "optional_parameter_declaration"
                && parent.child_by_field_name("default_value") == Some(node)
        })
    {
        return Some((value.byte_range(), true));
    }

    None
}

fn invocation(node: Node<'_>, source: &str) -> bool {
    let function = if node.kind() == "macro_type_specifier" && !node.has_error() {
        node.child_by_field_name("name")
    } else if node.kind() == "expression_statement"
        && node
            .child(node.child_count().saturating_sub(1))
            .is_some_and(|last| last.is_missing() && last.kind() == ";")
    {
        node.named_child(0)
            .filter(|call| call.kind() == "call_expression" && !call.has_error())
            .and_then(|call| call.child_by_field_name("function"))
    } else {
        None
    };

    let Some(function) = function.filter(|function| function.kind() == "identifier") else {
        return false;
    };

    let name = &source[function.byte_range()];
    let text = &source[node.byte_range()];

    name.bytes().any(|byte| byte.is_ascii_uppercase())
        && name
            .bytes()
            .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_')
        && !text.contains(['\r', '\n'])
        && source[..node.start_byte()]
            .rsplit('\n')
            .next()
            .is_some_and(|prefix| prefix.trim().is_empty())
        && source[node.end_byte()..]
            .split('\n')
            .next()
            .is_some_and(|suffix| suffix.trim().is_empty())
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

    separate && !(rules.enabled(Rule::Related) && related(left, right, source))
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

fn related(previous: Node<'_>, next: Node<'_>, source: &str) -> bool {
    let mut names = BTreeSet::new();
    written(previous, source, &mut names);

    if let Some(initializer) = next.child_by_field_name("initializer") {
        let mut shadowed = BTreeSet::new();

        if initializer.kind() == "declaration" {
            written(initializer, source, &mut shadowed);
        }

        names.retain(|name| !shadowed.contains(name));
    }

    !names.is_empty() && reads(next, source, &names)
}

fn written<'source>(node: Node<'_>, source: &'source str, names: &mut BTreeSet<&'source str>) {
    let field = match node.kind() {
        "init_declarator" => Some("declarator"),
        "assignment_expression" => Some("left"),
        "update_expression" => Some("argument"),
        _ => None,
    };

    if let Some(target) = field.and_then(|field| node.child_by_field_name(field)) {
        bindings(target, source, names);

        if let Some(right) = node.child_by_field_name("right") {
            written(right, source, names);
        }
    } else if node.kind() == "declaration" {
        let mut cursor = node.walk();

        for declarator in node.children_by_field_name("declarator", &mut cursor) {
            if declarator.kind() == "init_declarator" {
                written(declarator, source, names);
            } else {
                bindings(declarator, source, names);
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

        "field_expression" | "subscript_expression" | "pointer_expression" => {
            if let Some(argument) = node.child_by_field_name("argument") {
                bindings(argument, source, names);
            }
        }

        "pointer_declarator"
        | "reference_declarator"
        | "array_declarator"
        | "parenthesized_declarator" => {
            if let Some(declarator) = node
                .child_by_field_name("declarator")
                .or_else(|| node.named_child(0))
            {
                bindings(declarator, source, names);
            }
        }

        "structured_binding_declarator" | "parenthesized_expression" => {
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

        "field_expression" | "subscript_expression" | "pointer_expression" => {
            reads(node, source, names)
        }

        _ => {
            let mut cursor = node.walk();

            node.named_children(&mut cursor)
                .any(|child| target_reads(child, source, names))
        }
    }
}

fn reads(node: Node<'_>, source: &str, names: &BTreeSet<&str>) -> bool {
    if node.kind().starts_with("preproc_") {
        return false;
    }

    let field = |field| {
        node.child_by_field_name(field)
            .is_some_and(|child| reads(child, source, names))
    };

    match node.kind() {
        "identifier" => names.contains(&source[node.byte_range()]),

        "string_literal"
        | "raw_string_literal"
        | "char_literal"
        | "comment"
        | "function_definition"
        | "lambda_expression"
        | "compound_statement"
        | "else_clause"
        | "catch_clause"
        | "type_definition"
        | "attribute" => false,

        "field_expression" => field("argument"),
        "init_declarator" => field("value"),

        "assignment_expression" => {
            field("right")
                || node
                    .child_by_field_name("left")
                    .is_some_and(|left| target_reads(left, source, names))
                || (node
                    .child_by_field_name("operator")
                    .is_some_and(|operator| &source[operator.byte_range()] != "=")
                    && field("left"))
        }

        "for_range_loop" => field("right"),

        _ => {
            let mut cursor = node.walk();

            node.named_children(&mut cursor).any(|child| {
                !["type", "label"]
                    .into_iter()
                    .any(|field| node.child_by_field_name(field) == Some(child))
                    && reads(child, source, names)
            })
        }
    }
}
