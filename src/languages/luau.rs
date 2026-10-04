use std::collections::BTreeSet;

use vermis::{
    token::TokenKind,
    tree::{NodeIndex, NodeKind, NodeList, Tree},
};

use crate::configuration::{Rule, Rules};

fn text<'source>(node: NodeIndex, tree: &Tree<'_>, source: &'source str) -> &'source str {
    let span = tree.node(node).span;

    &source[span.start..span.end]
}

fn declaration<'source>(
    node: NodeIndex,
    tree: &Tree<'_>,
    source: &'source str,
) -> Option<&'source str> {
    let (NodeKind::Local { bindings, .. } | NodeKind::Constant { bindings, .. }) =
        &tree.node(node).kind
    else {
        return None;
    };

    let [binding] = tree.list(bindings) else {
        return None;
    };

    let NodeKind::Binding { name, .. } = tree.node(binding.node).kind else {
        return None;
    };

    Some(text(name, tree, source))
}

fn root<'source>(node: NodeIndex, tree: &Tree<'_>, source: &'source str) -> Option<&'source str> {
    match tree.node(node).kind {
        NodeKind::Field { receiver, .. } | NodeKind::Index { receiver, .. } => {
            root(receiver, tree, source)
        }

        NodeKind::Group { expression, .. } | NodeKind::Assertion { expression, .. } => {
            root(expression, tree, source)
        }

        NodeKind::Name { .. } => Some(text(node, tree, source)),
        _ => None,
    }
}

fn assignment<'source>(
    node: NodeIndex,
    tree: &Tree<'_>,
    source: &'source str,
) -> Option<&'source str> {
    if let NodeKind::CompoundAssignment { target, .. } = tree.node(node).kind {
        return matches!(
            tree.node(target).kind,
            NodeKind::Field { .. } | NodeKind::Index { .. }
        )
        .then(|| root(target, tree, source))
        .flatten();
    }

    let NodeKind::Assignment { targets, .. } = &tree.node(node).kind else {
        return None;
    };

    let (first, targets) = tree.list(targets).split_first()?;

    if !matches!(
        tree.node(first.node).kind,
        NodeKind::Field { .. } | NodeKind::Index { .. }
    ) {
        return None;
    }

    let name = root(first.node, tree, source)?;

    targets
        .iter()
        .all(|target| {
            matches!(
                tree.node(target.node).kind,
                NodeKind::Field { .. } | NodeKind::Index { .. }
            ) && root(target.node, tree, source) == Some(name)
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

fn require<'source>(
    node: NodeIndex,
    tree: &Tree<'_>,
    source: &'source str,
) -> Option<&'source str> {
    match tree.node(node).kind {
        NodeKind::Group { expression, .. } | NodeKind::Assertion { expression, .. } => {
            require(expression, tree, source)
        }

        NodeKind::Call { callee, arguments } if text(callee, tree, source) == "require" => {
            let NodeKind::Arguments { values, .. } = &tree.node(arguments).kind else {
                return None;
            };

            let value = tree.list(values).first()?.node;

            if !matches!(tree.node(value).kind, NodeKind::String { .. }) {
                return None;
            }

            let literal = text(value, tree, source);
            let quote = *literal.as_bytes().first()?;

            if !matches!(quote, b'\'' | b'"') || literal.as_bytes().last() != Some(&quote) {
                return None;
            }

            literal.get(1..literal.len() - 1)?.split('/').next()
        }

        _ => None,
    }
}

fn group<'source>(
    node: NodeIndex,
    tree: &Tree<'_>,
    source: &'source str,
) -> Option<Group<'source>> {
    match &tree.node(node).kind {
        NodeKind::Export { declaration, .. }
            if matches!(tree.node(*declaration).kind, NodeKind::TypeAlias { .. }) =>
        {
            Some(Group::Exported)
        }

        NodeKind::TypeAlias { annotation, .. } => Some(
            if matches!(
                tree.node(*annotation).kind,
                NodeKind::TypeName {
                    namespace: Some(_),
                    ..
                }
            ) {
                Group::Imported
            } else {
                Group::Local
            },
        ),

        NodeKind::Local { values, .. } | NodeKind::Constant { values, .. } => {
            let value = tree.list(values).first()?.node;

            if let NodeKind::MethodCall {
                receiver, method, ..
            } = tree.node(value).kind
                && text(receiver, tree, source) == "game"
                && text(method, tree, source) == "GetService"
            {
                return Some(Group::Service);
            }

            if let Some(alias) = require(value, tree, source) {
                return Some(Group::Require(alias));
            }

            if matches!(tree.node(value).kind, NodeKind::Table { .. })
                && declaration(node, tree, source)
                    .is_some_and(|name| name.starts_with(char::is_uppercase))
            {
                return Some(Group::Class);
            }

            None
        }

        NodeKind::Assignment { .. } | NodeKind::CompoundAssignment { .. }
            if assignment(node, tree, source)
                .is_some_and(|name| name.starts_with(char::is_uppercase)) =>
        {
            Some(Group::Class)
        }

        _ => None,
    }
}

fn multiline(node: NodeIndex, tree: &Tree<'_>, source: &str) -> bool {
    let tokens = &tree.node(node).tokens;

    tree.tokens[tokens.start.get()..tokens.end.get()]
        .iter()
        .any(|token| {
            token.kind == TokenKind::Whitespace
                && source[token.span.start..token.span.end].contains('\n')
        })
}

fn block(node: NodeIndex, tree: &Tree<'_>) -> Option<Rule> {
    match tree.node(node).kind {
        NodeKind::If { .. } => Some(Rule::Conditionals),
        NodeKind::While { .. } => Some(Rule::WhileLoops),
        NodeKind::Repeat { .. } => Some(Rule::RepeatLoops),
        NodeKind::NumericFor { .. } | NodeKind::GenericFor { .. } => Some(Rule::ForLoops),
        NodeKind::Do { .. } => Some(Rule::DoBlocks),
        NodeKind::Function { body: Some(_), .. } => Some(Rule::Functions),
        NodeKind::Class { .. } => Some(Rule::Classes),
        _ => None,
    }
}

fn expression(node: NodeIndex, tree: &Tree<'_>) -> Option<Rule> {
    let child = |node| expression(node, tree);
    let list = |nodes: &NodeList| tree.list(nodes).iter().find_map(|entry| child(entry.node));
    let children = |nodes: &[Option<NodeIndex>]| nodes.iter().flatten().copied().find_map(child);

    match &tree.node(node).kind {
        NodeKind::TypeAlias { .. } => Some(Rule::TypeAliases),
        NodeKind::Call { .. } | NodeKind::MethodCall { .. } => Some(Rule::Calls),
        NodeKind::Table { .. } => Some(Rule::Tables),

        NodeKind::CallStatement { call } => child(*call),

        NodeKind::Group { expression, .. } | NodeKind::TypeOf { expression, .. } => {
            child(*expression)
        }

        NodeKind::Instantiate {
            expression,
            arguments,
        } => child(*expression).or_else(|| child(*arguments)),

        NodeKind::Assertion {
            expression,
            annotation,
            ..
        } => child(*expression).or_else(|| child(*annotation)),

        NodeKind::Unary { operand, .. } => child(*operand),
        NodeKind::Binary { left, right, .. } => child(*left).or_else(|| child(*right)),
        NodeKind::Field { receiver, .. } => child(*receiver),
        NodeKind::Index { receiver, key, .. } => child(*receiver).or_else(|| child(*key)),

        NodeKind::Conditional {
            condition,
            truthy,
            falsy,
            ..
        } => children(&[Some(*condition), Some(*truthy), Some(*falsy)]),

        NodeKind::Binding {
            name, annotation, ..
        } => child(*name).or_else(|| annotation.and_then(child)),

        NodeKind::Variadic { annotation, .. } => annotation.and_then(child),

        NodeKind::TypeFunction {
            attributes,
            generics,
            parameters,
            returns,
            ..
        } => children(&[*attributes, *generics, Some(*parameters), Some(*returns)]),

        NodeKind::FunctionName { path, method, .. } => {
            list(path).or_else(|| method.and_then(child))
        }

        NodeKind::Parameters { parameters, .. } | NodeKind::Generics { parameters, .. } => {
            list(parameters)
        }

        NodeKind::Returns { annotation, .. }
        | NodeKind::TypeGroup { annotation, .. }
        | NodeKind::TypeOptional { annotation, .. }
        | NodeKind::VariadicType { annotation, .. } => child(*annotation),

        NodeKind::Attributes { attributes } | NodeKind::AttributeGroup { attributes, .. } => {
            list(attributes)
        }

        NodeKind::Attribute {
            name, arguments, ..
        }
        | NodeKind::TypeName {
            name, arguments, ..
        } => child(*name).or_else(|| arguments.and_then(child)),

        NodeKind::InstantiationArguments { arguments, .. } => child(*arguments),
        NodeKind::Generic { name, default, .. } => child(*name).or_else(|| default.and_then(child)),
        NodeKind::GenericPack { name, .. } => child(*name),
        NodeKind::TableField { key, value, .. } => key.and_then(child).or_else(|| child(*value)),

        NodeKind::TypeTable {
            element, fields, ..
        } => element.and_then(child).or_else(|| list(fields)),

        NodeKind::TypeField {
            key, annotation, ..
        }
        | NodeKind::TypeIndexer {
            key, annotation, ..
        } => child(*key).or_else(|| child(*annotation)),

        NodeKind::TypeParameter {
            name, annotation, ..
        } => child(*name).or_else(|| child(*annotation)),

        NodeKind::TypePack { types, .. } => list(types),
        NodeKind::TypeArguments { arguments, .. } => list(arguments),

        NodeKind::TypeUnion { left, right, .. }
        | NodeKind::TypeIntersection { left, right, .. } => {
            left.and_then(child).or_else(|| child(*right))
        }

        _ => statement(node, tree),
    }
}

fn statement(node: NodeIndex, tree: &Tree<'_>) -> Option<Rule> {
    let child = |node| expression(node, tree);
    let list = |nodes: &NodeList| tree.list(nodes).iter().find_map(|entry| child(entry.node));
    let children = |nodes: &[Option<NodeIndex>]| nodes.iter().flatten().copied().find_map(child);

    match &tree.node(node).kind {
        NodeKind::Root { block, .. } => child(*block),
        NodeKind::Block { statements } => list(statements),

        NodeKind::Local {
            bindings, values, ..
        }
        | NodeKind::Constant {
            bindings, values, ..
        } => list(bindings).or_else(|| list(values)),

        NodeKind::Assignment {
            targets, values, ..
        } => list(targets).or_else(|| list(values)),

        NodeKind::CompoundAssignment { target, value, .. } => {
            child(*target).or_else(|| child(*value))
        }

        NodeKind::Return { values, .. } | NodeKind::Arguments { values, .. } => list(values),

        NodeKind::Function {
            attributes,
            name,
            generics,
            parameters,
            returns,
            body,
            ..
        } => children(&[
            *attributes,
            *name,
            *generics,
            Some(*parameters),
            *returns,
            *body,
        ]),

        NodeKind::If {
            branches,
            otherwise,
            ..
        } => list(branches).or_else(|| otherwise.and_then(child)),

        NodeKind::Branch {
            condition, body, ..
        }
        | NodeKind::While {
            condition, body, ..
        } => child(*condition).or_else(|| child(*body)),

        NodeKind::Repeat {
            body, condition, ..
        } => child(*body).or_else(|| child(*condition)),

        NodeKind::NumericFor {
            binding,
            start,
            end,
            step,
            body,
            ..
        } => children(&[Some(*binding), Some(*start), Some(*end), *step, Some(*body)]),

        NodeKind::GenericFor {
            bindings,
            values,
            body,
            ..
        } => list(bindings)
            .or_else(|| list(values))
            .or_else(|| child(*body)),

        NodeKind::Do { body, .. } => child(*body),

        NodeKind::Export {
            attributes,
            declaration,
            ..
        } => attributes.and_then(child).or_else(|| child(*declaration)),

        NodeKind::Declaration { declaration, .. } => child(*declaration),

        NodeKind::Class {
            name,
            extends,
            members,
            ..
        } => children(&[Some(*name), *extends]).or_else(|| list(members)),

        NodeKind::Property { binding, .. } => child(*binding),
        NodeKind::Extends { superclass, .. } => child(*superclass),
        _ => None,
    }
}

fn complex(node: NodeIndex, tree: &Tree<'_>, source: &str, rules: &Rules) -> bool {
    if let Some(rule) = block(node, tree) {
        rules.enabled(rule)
    } else {
        let rule = expression(node, tree).unwrap_or(
            if matches!(
                tree.node(node).kind,
                NodeKind::Local { .. } | NodeKind::Constant { .. }
            ) {
                Rule::Declarations
            } else {
                Rule::Multiline
            },
        );

        rules.enabled(rule) && multiline(node, tree, source)
    }
}

fn reads(node: NodeIndex, tree: &Tree<'_>, source: &str, names: &BTreeSet<&str>) -> bool {
    let uses = |node| reads(node, tree, source, names);

    match &tree.node(node).kind {
        NodeKind::Name { .. } => names.contains(text(node, tree, source)),

        NodeKind::Local { values, .. }
        | NodeKind::Constant { values, .. }
        | NodeKind::Return { values, .. }
        | NodeKind::Arguments { values, .. }
        | NodeKind::GenericFor { values, .. } => {
            tree.list(values).iter().any(|entry| uses(entry.node))
        }

        NodeKind::Assignment {
            targets, values, ..
        } => {
            tree.list(values).iter().any(|entry| uses(entry.node))
                || tree.list(targets).iter().any(|target| {
                    !matches!(tree.node(target.node).kind, NodeKind::Name { .. })
                        && uses(target.node)
                })
        }

        NodeKind::CompoundAssignment { target, value, .. } => uses(*target) || uses(*value),

        NodeKind::If { branches, .. } => tree
            .list(branches)
            .first()
            .is_some_and(|entry| uses(entry.node)),

        NodeKind::Branch { condition, .. }
        | NodeKind::While { condition, .. }
        | NodeKind::Repeat { condition, .. } => uses(*condition),

        NodeKind::NumericFor {
            start, end, step, ..
        } => uses(*start) || uses(*end) || step.is_some_and(uses),

        NodeKind::CallStatement { call } => uses(*call),
        NodeKind::Call { callee, arguments } => uses(*callee) || uses(*arguments),

        NodeKind::MethodCall {
            receiver,
            arguments,
            ..
        } => uses(*receiver) || uses(*arguments),

        NodeKind::Field { receiver, .. } => uses(*receiver),
        NodeKind::Index { receiver, key, .. } => uses(*receiver) || uses(*key),

        NodeKind::Group { expression, .. }
        | NodeKind::Assertion { expression, .. }
        | NodeKind::Instantiate { expression, .. } => uses(*expression),

        NodeKind::Unary { operand, .. } => uses(*operand),
        NodeKind::Binary { left, right, .. } => uses(*left) || uses(*right),

        NodeKind::Conditional {
            condition,
            truthy,
            falsy,
            ..
        } => uses(*condition) || uses(*truthy) || uses(*falsy),

        NodeKind::Table { fields, .. } => tree.list(fields).iter().any(|entry| uses(entry.node)),

        NodeKind::TableField {
            key,
            value,
            opening,
            ..
        } => uses(*value) || (opening.is_some() && key.is_some_and(uses)),

        NodeKind::Interpolation { segments } => {
            tree.list(segments).iter().any(|entry| uses(entry.node))
        }

        _ => false,
    }
}

fn related(previous: NodeIndex, next: NodeIndex, tree: &Tree<'_>, source: &str) -> bool {
    let names: BTreeSet<_> = match &tree.node(previous).kind {
        NodeKind::Local { bindings, .. } | NodeKind::Constant { bindings, .. } => tree
            .list(bindings)
            .iter()
            .filter_map(|binding| {
                if let NodeKind::Binding { name, .. } = tree.node(binding.node).kind {
                    Some(text(name, tree, source))
                } else {
                    None
                }
            })
            .collect(),

        NodeKind::Assignment { targets, .. } => tree
            .list(targets)
            .iter()
            .filter_map(|target| root(target.node, tree, source))
            .collect(),

        NodeKind::CompoundAssignment { target, .. } => {
            root(*target, tree, source).into_iter().collect()
        }

        _ => return false,
    };

    !names.is_empty() && reads(next, tree, source, &names)
}

fn group_rule(group: &Group<'_>) -> Rule {
    match group {
        Group::Service => Rule::Services,
        Group::Require(_) => Rule::Requires,
        Group::Imported | Group::Local | Group::Exported => Rule::TypeGroups,
        Group::Class => Rule::ClassGroups,
    }
}

pub fn breathe(source: &str, rules: &Rules) -> Result<String, String> {
    let tree = vermis::parse(source.as_bytes());

    if !tree.diagnostics.is_empty() {
        return Err(tree
            .diagnostics
            .iter()
            .map(|diagnostic| format!("{} at byte {}", diagnostic.message, diagnostic.span.start))
            .collect::<Vec<_>>()
            .join("; "));
    }

    let mut insertions = BTreeSet::new();

    let top = &tree.node(tree.root).kind;

    for (index, node) in tree.nodes.iter().enumerate() {
        let NodeKind::Block { statements } = &node.kind else {
            continue;
        };

        let statements = tree.list(statements);

        for (position, pair) in statements.windows(2).enumerate() {
            let previous = pair[0].node;
            let next = pair[1].node;

            let populated = declaration(next, &tree, source).is_some_and(|name| {
                statements.get(position + 2).is_some_and(|following| {
                    assignment(following.node, &tree, source) == Some(name)
                })
            });

            let continuation = assignment(next, &tree, source).is_some_and(|name| {
                declaration(previous, &tree, source).or_else(|| assignment(previous, &tree, source))
                    == Some(name)
            });

            let groups = matches!(top, NodeKind::Root { block, .. } if block.get() == index)
                && matches!((group(previous, &tree, source), group(next, &tree, source)), (Some(left), Some(right)) if left != right && (rules.enabled(group_rule(&left)) || rules.enabled(group_rule(&right))));

            let separate = (rules.enabled(Rule::ReturnStatements)
                && matches!(tree.node(next).kind, NodeKind::Return { .. }))
                || complex(previous, &tree, source, rules)
                || complex(next, &tree, source, rules)
                || (!continuation
                    && (groups
                        || (rules.enabled(Rule::PopulatedDeclarations)
                            && (populated
                                || (assignment(previous, &tree, source).is_some()
                                    && declaration(next, &tree, source).is_some())
                                || (declaration(previous, &tree, source).is_some()
                                    && assignment(next, &tree, source).is_some())))));

            if !separate || (rules.enabled(Rule::Related) && related(previous, next, &tree, source))
            {
                continue;
            }

            let start = tree.node(previous).span.end;
            let end = tree.node(next).span.start;

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
    fn breathe(source: &str) -> Result<String, String> {
        super::breathe(source, &crate::configuration::Rules::default())
    }

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
            "declare function first(): number\ndeclare function second(): string\n",
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
