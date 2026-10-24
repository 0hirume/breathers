use std::{collections::BTreeSet, iter};

use miette::LabeledSpan;

use ra_ap_syntax::{
    AstNode as _, Edition, NodeOrToken, SourceFile, SyntaxKind, SyntaxNode, SyntaxToken,
    ast::{self, HasName as _},
};

use crate::{
    configuration::{Rule, Rules},
    error::Failure,
    spacing,
};

pub(crate) fn breathe(source: &str, rules: &Rules) -> Result<String, Failure> {
    let parsed = SourceFile::parse(source, Edition::CURRENT);
    let errors = parsed.errors();

    if !errors.is_empty() {
        return Err(Failure::syntax(
            source,
            errors
                .iter()
                .map(|error| {
                    let range = error.range();

                    LabeledSpan::new_with_span(
                        Some(error.to_string()),
                        usize::from(range.start())..usize::from(range.end()),
                    )
                })
                .collect(),
        ));
    }

    let mut insertions = BTreeSet::new();

    for container in parsed.tree().syntax().descendants().filter(|node| {
        items(node)
            || matches!(
                node.kind(),
                SyntaxKind::STMT_LIST | SyntaxKind::MATCH_ARM_LIST | SyntaxKind::VARIANT_LIST
            )
            || (matches!(
                node.kind(),
                SyntaxKind::RECORD_FIELD_LIST | SyntaxKind::TUPLE_FIELD_LIST
            ) && node
                .parent()
                .is_some_and(|parent| parent.kind() == SyntaxKind::STRUCT))
    }) {
        let statements: Vec<_> = container.children().collect();

        for pair in statements.windows(2) {
            let separate = if items(&container) {
                pair.iter().any(|item| complex(item, rules))
            } else if container.kind() == SyntaxKind::STMT_LIST {
                complex(&pair[0], rules)
                    || complex(&pair[1], rules)
                    || (if pair[1].kind() == SyntaxKind::RETURN_EXPR
                        || pair[1]
                            .children()
                            .any(|node| node.kind() == SyntaxKind::RETURN_EXPR)
                    {
                        rules.enabled(Rule::ReturnStatements)
                    } else {
                        ast::Expr::can_cast(pair[1].kind()) && rules.enabled(Rule::TailExpressions)
                    })
            } else {
                let rule = match container.kind() {
                    SyntaxKind::MATCH_ARM_LIST => Rule::MatchArms,
                    SyntaxKind::VARIANT_LIST => Rule::EnumVariants,
                    SyntaxKind::TUPLE_FIELD_LIST => Rule::TupleFields,
                    _ => Rule::StructFields,
                };

                rules.enabled(rule) && (multiline(&pair[0]) || multiline(&pair[1]))
            };

            if !separate
                || (container.kind() == SyntaxKind::STMT_LIST
                    && rules.enabled(Rule::Related)
                    && related(&pair[0], &pair[1]))
            {
                continue;
            }

            let start = usize::from(pair[0].text_range().end());

            let end = pair[1]
                .descendants_with_tokens()
                .filter_map(NodeOrToken::into_token)
                .find(|token| !token.kind().is_trivia())
                .map_or(usize::from(pair[1].text_range().start()), |token| {
                    usize::from(token.text_range().start())
                });

            let comments = iter::successors(
                pair[0].last_token().and_then(|token| token.next_token()),
                SyntaxToken::next_token,
            )
            .take_while(|token| usize::from(token.text_range().start()) < end)
            .filter(|token| token.kind() == SyntaxKind::COMMENT)
            .map(|token| {
                let range = token.text_range();

                usize::from(range.start())..usize::from(range.end())
            })
            .filter(|range| range.start >= start && range.end <= end);

            if let Some(boundary) = spacing::boundary(source, start, end, comments) {
                insertions.insert(boundary);
            }
        }
    }

    Ok(spacing::apply(source, insertions))
}

fn multiline(node: &SyntaxNode) -> bool {
    node.descendants_with_tokens().any(|element| {
        element.kind() == SyntaxKind::WHITESPACE && element.to_string().contains('\n')
    })
}

fn block(node: &SyntaxNode) -> Option<Rule> {
    match node.kind() {
        SyntaxKind::IF_EXPR => Some(Rule::Conditionals),
        SyntaxKind::MATCH_EXPR => Some(Rule::Matches),
        SyntaxKind::FOR_EXPR => Some(Rule::ForLoops),
        SyntaxKind::WHILE_EXPR => Some(Rule::WhileLoops),
        SyntaxKind::LOOP_EXPR => Some(Rule::LoopExpressions),

        SyntaxKind::FN
            if node
                .descendants()
                .any(|child| child.kind() == SyntaxKind::BLOCK_EXPR) =>
        {
            Some(Rule::Functions)
        }

        SyntaxKind::BLOCK_EXPR => Some(Rule::BlockExpressions),
        _ => None,
    }
}

fn expression(node: &SyntaxNode) -> Option<Rule> {
    match node.kind() {
        SyntaxKind::CALL_EXPR | SyntaxKind::METHOD_CALL_EXPR => Some(Rule::Calls),
        SyntaxKind::ARRAY_EXPR => Some(Rule::Arrays),
        _ => None,
    }
}

fn complex(node: &SyntaxNode, rules: &Rules) -> bool {
    if let Some(rule) = node.descendants().find_map(|node| block(&node)) {
        rules.enabled(rule)
    } else {
        let rule = if node.kind() == SyntaxKind::TYPE_ALIAS {
            Rule::TypeAliases
        } else {
            node.descendants()
                .find_map(|node| expression(&node))
                .unwrap_or(if node.kind() == SyntaxKind::LET_STMT {
                    Rule::Declarations
                } else {
                    Rule::Multiline
                })
        };

        rules.enabled(rule) && multiline(node)
    }
}

fn items(node: &SyntaxNode) -> bool {
    matches!(
        node.kind(),
        SyntaxKind::SOURCE_FILE
            | SyntaxKind::ITEM_LIST
            | SyntaxKind::ASSOC_ITEM_LIST
            | SyntaxKind::EXTERN_ITEM_LIST
    )
}

fn related(previous: &SyntaxNode, next: &SyntaxNode) -> bool {
    let mut names = BTreeSet::new();

    if let Some(statement) = ast::LetStmt::cast(previous.clone()) {
        if let Some(pattern) = statement.pat() {
            names.extend(
                pattern
                    .syntax()
                    .descendants()
                    .filter_map(ast::IdentPat::cast)
                    .filter_map(|binding| binding.name())
                    .map(|name| name.text().trim_start_matches("r#").to_owned()),
            );
        }
    } else if let Some(expression) = ast::ExprStmt::cast(previous.clone())
        .and_then(|statement| statement.expr())
        .and_then(|expression| ast::BinExpr::cast(expression.syntax().clone()))
        && matches!(expression.op_kind(), Some(ast::BinaryOp::Assignment { .. }))
        && let Some(left) = expression.lhs()
    {
        targets(left.syntax(), &mut names);
    }

    !names.is_empty() && reads(next, &names)
}

fn targets(node: &SyntaxNode, names: &mut BTreeSet<String>) {
    match node.kind() {
        SyntaxKind::PATH_EXPR => {
            names.insert(node.text().to_string().trim_start_matches("r#").to_owned());
        }

        SyntaxKind::FIELD_EXPR
        | SyntaxKind::INDEX_EXPR
        | SyntaxKind::PREFIX_EXPR
        | SyntaxKind::PAREN_EXPR => {
            if let Some(receiver) = node
                .children()
                .find(|child| ast::Expr::can_cast(child.kind()))
            {
                targets(&receiver, names);
            }
        }

        SyntaxKind::TUPLE_EXPR | SyntaxKind::ARRAY_EXPR => {
            for child in node.children() {
                targets(&child, names);
            }
        }

        _ => {}
    }
}

fn reads(node: &SyntaxNode, names: &BTreeSet<String>) -> bool {
    if ast::Type::can_cast(node.kind()) || ast::Pat::can_cast(node.kind()) {
        return false;
    }

    match node.kind() {
        SyntaxKind::PATH_EXPR => names.contains(node.text().to_string().trim_start_matches("r#")),

        SyntaxKind::FN
        | SyntaxKind::CLOSURE_EXPR
        | SyntaxKind::BLOCK_EXPR
        | SyntaxKind::MATCH_ARM_LIST
        | SyntaxKind::MACRO_CALL
        | SyntaxKind::TOKEN_TREE
        | SyntaxKind::TYPE_ALIAS
        | SyntaxKind::GENERIC_ARG_LIST => false,

        SyntaxKind::LET_STMT => ast::LetStmt::cast(node.clone())
            .and_then(|statement| statement.initializer())
            .is_some_and(|value| reads(value.syntax(), names)),

        SyntaxKind::IF_EXPR => ast::IfExpr::cast(node.clone())
            .and_then(|expression| expression.condition())
            .is_some_and(|condition| reads(condition.syntax(), names)),

        SyntaxKind::RECORD_EXPR_FIELD => {
            let field = ast::RecordExprField::cast(node.clone()).unwrap();

            field.expr().map_or_else(
                || {
                    field
                        .name_ref()
                        .is_some_and(|name| names.contains(name.text().trim_start_matches("r#")))
                },
                |expression| reads(expression.syntax(), names),
            )
        }

        SyntaxKind::BIN_EXPR => {
            let expression = ast::BinExpr::cast(node.clone()).unwrap();

            expression
                .rhs()
                .is_some_and(|right| reads(right.syntax(), names))
                || expression.lhs().is_some_and(|left| {
                    if expression.op_kind() == Some(ast::BinaryOp::Assignment { op: None }) {
                        target_reads(left.syntax(), names)
                    } else {
                        reads(left.syntax(), names)
                    }
                })
        }

        _ => node.children().any(|child| reads(&child, names)),
    }
}

fn target_reads(node: &SyntaxNode, names: &BTreeSet<String>) -> bool {
    match node.kind() {
        SyntaxKind::PATH_EXPR => false,

        SyntaxKind::FIELD_EXPR | SyntaxKind::INDEX_EXPR | SyntaxKind::PREFIX_EXPR => {
            reads(node, names)
        }

        _ => node.children().any(|child| target_reads(&child, names)),
    }
}
