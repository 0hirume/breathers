use std::collections::BTreeSet;

use ra_ap_syntax::{AstNode, Edition, SourceFile, SyntaxKind, SyntaxNode, ast};

fn multiline(node: &SyntaxNode) -> bool {
    node.descendants_with_tokens().any(|element| {
        element.kind() == SyntaxKind::WHITESPACE && element.to_string().contains('\n')
    })
}

fn needs_spacing(statement: &SyntaxNode) -> bool {
    statement.descendants().any(|node| {
        matches!(
            node.kind(),
            SyntaxKind::IF_EXPR
                | SyntaxKind::MATCH_EXPR
                | SyntaxKind::FOR_EXPR
                | SyntaxKind::WHILE_EXPR
                | SyntaxKind::LOOP_EXPR
                | SyntaxKind::BLOCK_EXPR
        )
    }) || multiline(statement)
}

pub fn breathe(source: &str) -> Result<String, String> {
    let parsed = SourceFile::parse(source, Edition::CURRENT);
    let errors = parsed.errors();

    if !errors.is_empty() {
        return Err(errors
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("; "));
    }

    let mut insertions = BTreeSet::new();

    for block in parsed.tree().syntax().descendants().filter(|node| {
        matches!(
            node.kind(),
            SyntaxKind::STMT_LIST | SyntaxKind::MATCH_ARM_LIST | SyntaxKind::VARIANT_LIST
        ) || (matches!(
            node.kind(),
            SyntaxKind::RECORD_FIELD_LIST | SyntaxKind::TUPLE_FIELD_LIST
        ) && node
            .parent()
            .is_some_and(|parent| parent.kind() == SyntaxKind::STRUCT))
    }) {
        let statements: Vec<_> = block.children().collect();

        for pair in statements.windows(2) {
            let separate = if block.kind() == SyntaxKind::STMT_LIST {
                needs_spacing(&pair[0])
                    || needs_spacing(&pair[1])
                    || ast::Expr::can_cast(pair[1].kind())
                    || pair[1]
                        .children()
                        .any(|node| node.kind() == SyntaxKind::RETURN_EXPR)
            } else {
                multiline(&pair[0]) || multiline(&pair[1])
            };

            if !separate {
                continue;
            }

            let start = usize::from(pair[0].text_range().end());

            let end = pair[1]
                .descendants_with_tokens()
                .filter_map(ra_ap_syntax::NodeOrToken::into_token)
                .find(|token| !token.kind().is_trivia())
                .map_or(usize::from(pair[1].text_range().start()), |token| {
                    usize::from(token.text_range().start())
                });

            let mut boundary = None;
            let mut already_spaced = false;

            for element in std::iter::successors(
                pair[0].last_token().and_then(|token| token.next_token()),
                ra_ap_syntax::SyntaxToken::next_token,
            )
            .take_while(|element| usize::from(element.text_range().start()) < end)
            {
                if element.kind() != SyntaxKind::WHITESPACE {
                    continue;
                }

                let range = element.text_range();
                let offset = usize::from(range.start());

                if offset < start || usize::from(range.end()) > end {
                    continue;
                }

                let whitespace = &source[offset..usize::from(range.end())];

                if whitespace.bytes().filter(|byte| *byte == b'\n').count() > 1 {
                    already_spaced = true;
                }

                if boundary.is_none()
                    && let Some(newline) = whitespace.find('\n')
                {
                    boundary = Some(offset + newline + 1);
                }
            }

            if !already_spaced && let Some(boundary) = boundary {
                insertions.insert(boundary);
            }
        }
    }

    Ok(crate::spacing::apply(source, insertions))
}
