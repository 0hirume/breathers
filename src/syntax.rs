use tree_sitter::Node;

use crate::configuration::Rule;

pub fn rule(
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

pub fn multiline(node: Node<'_>, source: &str, opaque: fn(Node<'_>) -> bool) -> bool {
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

fn bindings<'source>(
    node: Node<'_>,
    source: &'source str,
    names: &mut std::collections::BTreeSet<&'source str>,
) {
    match node.kind() {
        "identifier" | "shorthand_property_identifier_pattern" => {
            names.insert(&source[node.byte_range()]);
        }

        "pair_pattern" => {
            if let Some(value) = node.child_by_field_name("value") {
                bindings(value, source, names);
            }
        }

        "assignment_pattern" => {
            if let Some(left) = node.child_by_field_name("left") {
                bindings(left, source, names);
            }
        }

        "member_expression"
        | "attribute"
        | "field_expression"
        | "dot_index_expression"
        | "subscript_expression"
        | "subscript"
        | "bracket_index_expression"
        | "pointer_expression" => {
            for field in ["object", "argument", "value", "table"] {
                if let Some(receiver) = node.child_by_field_name(field) {
                    bindings(receiver, source, names);
                    break;
                }
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

        "variable_list"
        | "object_pattern"
        | "array_pattern"
        | "pattern_list"
        | "tuple_pattern"
        | "list_pattern"
        | "tuple"
        | "list"
        | "rest_pattern"
        | "list_splat_pattern"
        | "dictionary_splat_pattern"
        | "structured_binding_declarator"
        | "parenthesized_expression" => {
            let mut cursor = node.walk();

            for child in node.named_children(&mut cursor) {
                bindings(child, source, names);
            }
        }

        _ => {}
    }
}

fn written<'source>(
    node: Node<'_>,
    source: &'source str,
    names: &mut std::collections::BTreeSet<&'source str>,
) {
    let field = match node.kind() {
        "variable_declarator" => Some("name"),
        "init_declarator" => Some("declarator"),

        "assignment"
        | "assignment_expression"
        | "augmented_assignment"
        | "augmented_assignment_expression" => Some("left"),

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
    } else if matches!(
        node.kind(),
        "expression_statement"
            | "variable_declaration"
            | "lexical_declaration"
            | "assignment_statement"
    ) {
        let mut cursor = node.walk();

        for child in node.named_children(&mut cursor) {
            if child.kind() == "variable_list" {
                bindings(child, source, names);
            } else {
                written(child, source, names);
            }
        }
    }
}

fn target_reads(node: Node<'_>, source: &str, names: &std::collections::BTreeSet<&str>) -> bool {
    match node.kind() {
        "identifier" | "shorthand_property_identifier_pattern" => false,

        "member_expression"
        | "attribute"
        | "field_expression"
        | "dot_index_expression"
        | "subscript_expression"
        | "subscript"
        | "bracket_index_expression"
        | "pointer_expression" => reads(node, source, names),

        _ => {
            let mut cursor = node.walk();

            node.named_children(&mut cursor)
                .any(|child| target_reads(child, source, names))
        }
    }
}

fn opaque_reference(node: Node<'_>) -> bool {
    matches!(
        node.kind(),
        "string_literal"
            | "raw_string_literal"
            | "char_literal"
            | "comment"
            | "regex"
            | "hash_bang_line"
            | "function_definition"
            | "function_declaration"
            | "generator_function_declaration"
            | "function_expression"
            | "function"
            | "arrow_function"
            | "lambda"
            | "lambda_expression"
            | "class_definition"
            | "class_declaration"
            | "class"
            | "decorated_definition"
            | "method_definition"
            | "block"
            | "compound_statement"
            | "statement_block"
            | "class_body"
            | "interface_body"
            | "else_clause"
            | "else_statement"
            | "elseif_statement"
            | "elif_clause"
            | "catch_clause"
            | "finally_clause"
            | "type_annotation"
            | "type_arguments"
            | "type_parameters"
            | "type_alias_declaration"
            | "type_alias_statement"
            | "variable_list"
            | "jsx_closing_element"
            | "jsx_namespace_name"
            | "import_statement"
            | "import_from_statement"
            | "type_definition"
    ) || node.kind().starts_with("preproc_")
}

fn comprehension_reads(
    node: Node<'_>,
    source: &str,
    names: &std::collections::BTreeSet<&str>,
) -> bool {
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
                let mut shadowed = std::collections::BTreeSet::new();
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

fn reads(node: Node<'_>, source: &str, names: &std::collections::BTreeSet<&str>) -> bool {
    if opaque_reference(node) {
        return false;
    }

    let field = |field| {
        node.child_by_field_name(field)
            .is_some_and(|child| reads(child, source, names))
    };

    match node.kind() {
        "identifier" | "shorthand_property_identifier" => {
            names.contains(&source[node.byte_range()])
        }

        "string" => {
            let mut cursor = node.walk();

            node.named_children(&mut cursor)
                .any(|child| child.kind() == "interpolation" && reads(child, source, names))
        }

        "list_comprehension"
        | "set_comprehension"
        | "dictionary_comprehension"
        | "generator_expression" => comprehension_reads(node, source, names),

        "jsx_opening_element" | "jsx_self_closing_element" => {
            let mut cursor = node.walk();

            node.named_children(&mut cursor).any(|child| {
                !(Some(child) == node.child_by_field_name("name")
                    && child.kind() == "identifier"
                    && source[child.byte_range()]
                        .starts_with(|character: char| character.is_ascii_lowercase()))
                    && reads(child, source, names)
            })
        }

        "member_expression" | "attribute" => field("object"),
        "field_expression" => field("argument"),
        "dot_index_expression" | "method_index_expression" => field("table"),

        "field" => {
            field("value")
                || (node.child(0).is_some_and(|child| child.kind() == "[") && field("name"))
        }

        "keyword_argument" | "variable_declarator" | "init_declarator" => field("value"),

        "assignment" | "assignment_expression" => {
            field("right")
                || node
                    .child_by_field_name("left")
                    .is_some_and(|left| target_reads(left, source, names))
                || (node
                    .child_by_field_name("operator")
                    .is_some_and(|operator| &source[operator.byte_range()] != "=")
                    && field("left"))
        }

        "augmented_assignment" | "augmented_assignment_expression" => {
            field("right") || field("left")
        }

        "for_in_statement" | "for_range_loop" => field("right"),
        "for_statement" if node.child_by_field_name("right").is_some() => field("right"),
        "for_numeric_clause" => field("start") || field("end") || field("step"),

        "assignment_statement" => {
            let mut cursor = node.walk();

            node.named_children(&mut cursor).any(|child| {
                if child.kind() == "variable_list" {
                    target_reads(child, source, names)
                } else {
                    reads(child, source, names)
                }
            })
        }

        _ => {
            let mut cursor = node.walk();

            node.named_children(&mut cursor).any(|child| {
                !["type", "return_type", "alias", "label"]
                    .into_iter()
                    .any(|field| node.child_by_field_name(field) == Some(child))
                    && reads(child, source, names)
            })
        }
    }
}

pub fn related(previous: Node<'_>, next: Node<'_>, source: &str) -> bool {
    let mut names = std::collections::BTreeSet::new();
    written(previous, source, &mut names);

    if let Some(initializer) = next.child_by_field_name("initializer") {
        let mut shadowed = std::collections::BTreeSet::new();

        if matches!(
            initializer.kind(),
            "declaration" | "lexical_declaration" | "variable_declaration"
        ) {
            written(initializer, source, &mut shadowed);
        }

        names.retain(|name| !shadowed.contains(name));
    }

    !names.is_empty() && reads(next, source, &names)
}

pub fn boundary(source: &str, start: usize, end: usize, comments: &[Node<'_>]) -> Option<usize> {
    let mut position = start;
    let mut boundary = None;

    for (stop, next) in comments
        .iter()
        .map(|node| (node.start_byte(), node.end_byte()))
        .chain(std::iter::once((end, end)))
    {
        let gap = &source[position..stop];

        if gap.bytes().filter(|byte| *byte == b'\n').count() > 1 {
            return None;
        }

        if let Some(newline) = gap.find('\n') {
            if gap[..newline].trim_end_matches('\r').ends_with('\\') {
                return None;
            }

            boundary.get_or_insert(position + newline + 1);
        }

        position = next;
    }

    boundary
}
