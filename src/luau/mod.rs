use std::collections::BTreeSet;

use vermis::{Kind, Parts, TokenKind, Tree, View};

fn text<'source>(node: View<'_, '_>, source: &'source str) -> &'source str {
    &source[node.span().start..node.span().end]
}

fn declaration<'source>(node: View<'_, '_>, source: &'source str) -> Option<&'source str> {
    let Parts::Local { mut bindings, .. } = node.parts()? else {
        return None;
    };
    let binding = bindings.next()?;
    if bindings.next().is_some() {
        return None;
    }
    let Parts::Binding { name, .. } = binding.parts()? else {
        return None;
    };
    Some(text(name, source))
}

fn root<'source>(node: View<'_, '_>, source: &'source str) -> Option<&'source str> {
    match node.parts()? {
        Parts::Field { receiver, .. } | Parts::Index { receiver, .. } => root(receiver, source),
        Parts::Group { expression } | Parts::Assertion { expression, .. } => {
            root(expression, source)
        }
        _ if node.kind() == Kind::Name => Some(text(node, source)),
        _ => None,
    }
}

fn assignment<'source>(node: View<'_, '_>, source: &'source str) -> Option<&'source str> {
    let Parts::Assignment { mut targets, .. } = node.parts()? else {
        return None;
    };
    let first = targets.next()?;
    if !matches!(first.kind(), Kind::Field | Kind::Index) {
        return None;
    }
    let name = root(first, source)?;
    targets
        .all(|target| {
            matches!(target.kind(), Kind::Field | Kind::Index) && root(target, source) == Some(name)
        })
        .then_some(name)
}

#[derive(PartialEq)]
enum Group<'source> {
    Service,
    Require(&'source str),
    Imported,
    Local,
    Exported,
    Class,
}

fn require<'source>(node: View<'_, '_>, source: &'source str) -> Option<&'source str> {
    match node.parts()? {
        Parts::Group { expression } | Parts::Assertion { expression, .. } => {
            require(expression, source)
        }
        Parts::Call { callee, arguments } if text(callee, source) == "require" => {
            let value = arguments.children().next()?;
            if value.kind() != Kind::String {
                return None;
            }
            let literal = text(value, source);
            let quote = *literal.as_bytes().first()?;
            if !matches!(quote, b'\'' | b'"') || literal.as_bytes().last() != Some(&quote) {
                return None;
            }
            literal.get(1..literal.len() - 1)?.split('/').next()
        }
        _ => None,
    }
}

fn group<'source>(node: View<'_, '_>, source: &'source str) -> Option<Group<'source>> {
    match node.parts()? {
        Parts::Export { declaration, .. } if declaration.kind() == Kind::TypeAlias => {
            Some(Group::Exported)
        }
        Parts::TypeAlias { annotation, .. } => Some(
            if matches!(
                annotation.parts(),
                Some(Parts::TypeName {
                    namespace: Some(_),
                    ..
                })
            ) {
                Group::Imported
            } else {
                Group::Local
            },
        ),
        Parts::Local { mut values, .. } => {
            let value = values.next()?;
            if let Some(Parts::MethodCall {
                receiver, method, ..
            }) = value.parts()
                && text(receiver, source) == "game"
                && text(method, source) == "GetService"
            {
                return Some(Group::Service);
            }
            if let Some(alias) = require(value, source) {
                return Some(Group::Require(alias));
            }
            if value.kind() == Kind::Table
                && declaration(node, source)
                    .is_some_and(|name| name.starts_with(char::is_uppercase))
            {
                return Some(Group::Class);
            }
            None
        }
        Parts::Assignment { .. }
            if assignment(node, source)
                .is_some_and(|name| name.starts_with(char::is_uppercase)) =>
        {
            Some(Group::Class)
        }
        _ => None,
    }
}

fn multiline(node: View<'_, '_>, tree: &Tree<'_>, source: &str) -> bool {
    let span = node.span();
    let start = tree
        .tokens
        .partition_point(|token| token.span.end <= span.start);
    tree.tokens[start..]
        .iter()
        .take_while(|token| token.span.end <= span.end)
        .any(|token| {
            token.kind == TokenKind::Whitespace
                && source[token.span.start..token.span.end].contains('\n')
        })
}

fn block(node: View<'_, '_>) -> bool {
    matches!(
        node.kind(),
        Kind::If
            | Kind::While
            | Kind::Repeat
            | Kind::NumericFor
            | Kind::GenericFor
            | Kind::Do
            | Kind::Function
            | Kind::LocalFunction
            | Kind::TypeFunction
            | Kind::Class
    )
}

pub fn breathe(source: &str) -> Result<String, String> {
    let tree = vermis::parse(source.into());
    if !tree.diagnostics.is_empty() {
        return Err(tree
            .diagnostics
            .iter()
            .map(|diagnostic| format!("{} at byte {}", diagnostic.message, diagnostic.span.start))
            .collect::<Vec<_>>()
            .join("; "));
    }
    let mut insertions = BTreeSet::new();
    let top = tree.children[tree.nodes[tree.root].children.clone()]
        .first()
        .copied();
    for (index, node) in tree
        .nodes
        .iter()
        .enumerate()
        .filter(|(_, node)| node.kind == Kind::Block)
    {
        let statements: Vec<_> = tree.children[node.children.clone()]
            .iter()
            .filter_map(|index| tree.view(*index))
            .collect();
        for (position, pair) in statements.windows(2).enumerate() {
            let previous = pair[0];
            let next = pair[1];
            let populated = declaration(next, source).is_some_and(|name| {
                statements
                    .get(position + 2)
                    .is_some_and(|following| assignment(*following, source) == Some(name))
            });
            let continuation = assignment(next, source).is_some_and(|name| {
                declaration(previous, source).or_else(|| assignment(previous, source)) == Some(name)
            });
            let groups = Some(index) == top
                && matches!((group(previous, source), group(next, source)), (Some(left), Some(right)) if left != right);
            let separate = next.kind() == Kind::Return
                || block(previous)
                || block(next)
                || multiline(previous, &tree, source)
                || multiline(next, &tree, source)
                || (!continuation
                    && (populated
                        || groups
                        || (assignment(previous, source).is_some()
                            && declaration(next, source).is_some())
                        || (declaration(previous, source).is_some()
                            && assignment(next, source).is_some())));
            if !separate {
                continue;
            }
            let start = previous.span().end;
            let end = next.span().start;
            let first = tree
                .tokens
                .partition_point(|token| token.span.start < start);
            let mut boundary = None;
            let mut spaced = false;
            for token in tree.tokens[first..]
                .iter()
                .take_while(|token| token.span.end <= end)
            {
                if token.kind != TokenKind::Whitespace {
                    continue;
                }
                let whitespace = &source[token.span.start..token.span.end];
                spaced |= whitespace.bytes().filter(|byte| *byte == b'\n').count() > 1;
                if boundary.is_none()
                    && let Some(newline) = whitespace.find('\n')
                {
                    boundary = Some(token.span.start + newline + 1);
                }
            }
            if !spaced && let Some(boundary) = boundary {
                insertions.insert(boundary);
            }
        }
    }
    Ok(crate::spacing::apply(source, insertions))
}

#[cfg(test)]
mod tests {
    use super::breathe;

    #[test]
    fn spaces_blocks_multiline_values_and_returns() {
        let source = "local first = 1\nlocal second = 2 -- trailing\n-- attached\nif first < second then\n    work()\nend\nlocal values = {\n    first,\n    second,\n}\nsave(values)\nreturn values\n";
        let expected = source
            .replace("-- trailing\n", "-- trailing\n\n")
            .replace("end\n", "end\n\n")
            .replace("}\n", "}\n\n")
            .replace("return values", "\nreturn values");
        assert_eq!(breathe(source).unwrap(), expected);
        assert_eq!(breathe(&expected).unwrap(), expected);
        assert_eq!(
            breathe(&source.replace('\n', "\r\n")).unwrap(),
            expected.replace('\n', "\r\n")
        );
        assert!(breathe("local value = {").is_err());
    }

    #[test]
    fn groups_services_imports_types_and_populated_declarations() {
        let source = "local players = game:GetService(\"Players\")\nlocal storage = game:GetService(\"ReplicatedStorage\")\nlocal first = require(\"@first/one\")\nlocal second = require(\"@first/two\")\nlocal other = require(\"@other/one\")\ntype First = first.First\ntype Second = string\nexport type Third = number\nlocal before = 1\nlocal values = {}\nvalues.first = before\nvalues.second = 2\nlocal after = 3\n";
        let expected = source
            .replace("local first =", "\nlocal first =")
            .replace("local other =", "\nlocal other =")
            .replace("type First =", "\ntype First =")
            .replace("type Second =", "\ntype Second =")
            .replace("export type Third =", "\nexport type Third =")
            .replace("local values =", "\nlocal values =")
            .replace("local after =", "\nlocal after =");
        assert_eq!(breathe(source).unwrap(), expected);
        assert_eq!(breathe(&expected).unwrap(), expected);
    }

    #[test]
    fn preserves_literals_comments_and_compact_code() {
        for source in [
            "local text = [=[first\nsecond]=]\nwork()\n",
            "local text = `first {value} second`\nwork()\n",
            "local value = 1 --[[first\nsecond]]\nwork()\n",
            "local first = 1; if first > 0 then work() end; finish()\n",
            "local first = 1\nlocal second = 2\n",
        ] {
            assert_eq!(breathe(source).unwrap(), source);
        }
        let source = "local function example()\n    work()\n    return 1\nend\n";
        assert_eq!(
            breathe(source).unwrap(),
            source.replace("    return", "\n    return")
        );
    }
}
