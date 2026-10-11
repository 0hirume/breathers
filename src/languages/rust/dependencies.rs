use std::collections::BTreeSet;

use ra_ap_syntax::{
    AstNode as _, SyntaxKind, SyntaxNode,
    ast::{self, HasName as _},
};

pub(super) fn related(previous: &SyntaxNode, next: &SyntaxNode) -> bool {
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
