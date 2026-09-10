use std::{collections::BTreeSet, env, fs, process::ExitCode};

use ra_ap_syntax::{AstNode, Edition, SourceFile, SyntaxKind, SyntaxNode, ast};

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
    }) || statement.descendants_with_tokens().any(|element| {
        element.kind() == SyntaxKind::WHITESPACE && element.to_string().contains('\n')
    })
}

fn breathe(source: &str) -> Result<String, String> {
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

    for block in parsed
        .tree()
        .syntax()
        .descendants()
        .filter_map(ast::StmtList::cast)
    {
        let statements: Vec<_> = block.syntax().children().collect();

        for pair in statements.windows(2) {
            if !needs_spacing(&pair[0]) && !needs_spacing(&pair[1]) {
                continue;
            }

            let start = usize::from(pair[0].text_range().end());
            let end = usize::from(pair[1].text_range().start());
            let mut boundary = None;
            let mut already_spaced = false;

            for element in std::iter::successors(pair[0].next_sibling_or_token(), |element| {
                element.next_sibling_or_token()
            })
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

    let mut output = String::new();
    let mut previous = 0;

    for offset in insertions {
        output.push_str(&source[previous..offset]);

        output.push_str(if source[..offset].ends_with("\r\n") {
            "\r\n"
        } else {
            "\n"
        });

        previous = offset;
    }

    output.push_str(&source[previous..]);
    Ok(output)
}

fn run() -> Result<(), String> {
    let paths: Vec<_> = env::args_os().skip(1).collect();

    if paths.is_empty() {
        return Err("usage: breather <files.rs...>".into());
    }

    let mut changes = Vec::new();

    for path in paths {
        let path = std::path::PathBuf::from(path);

        let source =
            fs::read_to_string(&path).map_err(|error| format!("{}: {error}", path.display()))?;

        let output = breathe(&source).map_err(|error| format!("{}: {error}", path.display()))?;

        if source != output {
            changes.push((path, output));
        }
    }

    for (path, output) in changes {
        fs::write(&path, output).map_err(|error| format!("{}: {error}", path.display()))?;
    }

    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::breathe;

    #[test]
    fn spaces_statements_without_changing_tokens() {
        let source = "fn example() {\n    let first = 1;\n    let second = 2; // trailing\n    // attached\n    if first < second {\n        work();\n    }\n    let values = [\n        first,\n        second,\n    ];\n    finish();\n}\n";
        let expected = "fn example() {\n    let first = 1;\n    let second = 2; // trailing\n\n    // attached\n    if first < second {\n        work();\n    }\n\n    let values = [\n        first,\n        second,\n    ];\n\n    finish();\n}\n";
        assert_eq!(breathe(source).unwrap(), expected);
        assert_eq!(breathe(expected).unwrap(), expected);

        assert_eq!(
            breathe(&source.replace('\n', "\r\n")).unwrap(),
            expected.replace('\n', "\r\n")
        );
    }

    #[test]
    fn preserves_literals_macros_and_compact_code() {
        for source in [
            "fn example() { one(); if ready() { two(); } three(); }\n",
            "fn example() {\n    let text = r#\"first\nsecond\"#;\n    finish();\n}\n",
            "macro_rules! example { () => { first(); if ready() { second(); } third(); }; }\n",
            "fn example() {\n    first();\n\n    // attached\n    if ready() {}\n}\n",
        ] {
            assert_eq!(breathe(source).unwrap(), source);
        }

        assert!(breathe("fn broken( {").is_err());
    }

    #[test]
    fn spaces_control_flow_and_tail_expressions() {
        for statement in [
            "if ready() {} else {}",
            "match value { _ => () }",
            "for value in values {}",
            "while ready() {}",
            "loop { break; }",
            "let value = if ready() { 1 } else { 2 };",
        ] {
            let source =
                format!("fn example() {{\n    first();\n    {statement}\n    result\n}}\n");

            let expected =
                format!("fn example() {{\n    first();\n\n    {statement}\n\n    result\n}}\n");

            assert_eq!(breathe(&source).unwrap(), expected);
        }
    }
}
