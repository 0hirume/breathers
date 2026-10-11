use std::iter::from_fn;

use tree_sitter::Node;

pub(super) fn related(previous: Node<'_>, next: Node<'_>, source: &str) -> bool {
    let previous = statement(previous);

    previous.kind() == "assignment_expression"
        && previous
            .named_child(0)
            .is_some_and(|target| dependencies(target, next, source))
}

pub(super) fn statement(mut node: Node<'_>) -> Node<'_> {
    while matches!(node.kind(), "pipeline" | "assignment_value") {
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
