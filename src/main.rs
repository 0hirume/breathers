use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
    process::ExitCode,
};

use clap::Parser;

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

#[derive(Parser)]
/// Give Rust code breathing room.
struct Arguments {
    /// Rust files or directories to format recursively.
    // An omitted path means the entire current directory.
    #[arg(default_value = ".")]
    paths: Vec<PathBuf>,
}

fn collect(
    path: &Path,
    files: &mut BTreeSet<PathBuf>,
    directories: &mut BTreeSet<PathBuf>,
) -> Result<(), String> {
    let canonical =
        fs::canonicalize(path).map_err(|error| format!("{}: {error}", path.display()))?;

    let metadata =
        fs::metadata(&canonical).map_err(|error| format!("{}: {error}", path.display()))?;

    if metadata.is_dir() {
        if !directories.insert(canonical.clone()) {
            return Ok(());
        }

        for entry in
            fs::read_dir(&canonical).map_err(|error| format!("{}: {error}", path.display()))?
        {
            let entry = entry.map_err(|error| format!("{}: {error}", path.display()))?;

            let kind = entry
                .file_type()
                .map_err(|error| format!("{}: {error}", entry.path().display()))?;

            if kind.is_symlink()
                || (kind.is_dir() && matches!(entry.file_name().to_str(), Some(".git" | "target")))
            {
                continue;
            }

            if kind.is_dir()
                || (kind.is_file()
                    && entry
                        .path()
                        .extension()
                        .is_some_and(|extension| extension == "rs"))
            {
                collect(&entry.path(), files, directories)?;
            }
        }
    } else if metadata.is_file() {
        files.insert(canonical);
    } else {
        return Err(format!("{}: expected a file or directory", path.display()));
    }

    Ok(())
}

fn run() -> Result<(), String> {
    let arguments = Arguments::parse();
    let mut files = BTreeSet::new();
    let mut directories = BTreeSet::new();

    for path in arguments.paths {
        collect(&path, &mut files, &mut directories)?;
    }

    let mut changes = Vec::new();

    for path in files {
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
    use super::{Arguments, breathe, collect};
    use clap::Parser;
    use std::{collections::BTreeSet, fs, path::PathBuf};

    #[test]
    fn discovers_rust_files_and_deduplicates_paths() {
        let root = std::env::temp_dir().join(format!(
            "breathers-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));

        fs::create_dir(&root).unwrap();

        for directory in ["src", "src/nested", "target", ".git"] {
            fs::create_dir(root.join(directory)).unwrap();
        }

        for file in [
            "root.rs",
            "src/main.rs",
            "src/nested/module.rs",
            "src/notes.txt",
            "target/generated.rs",
            ".git/ignored.rs",
        ] {
            fs::write(root.join(file), "fn example() {}\n").unwrap();
        }

        let expected: BTreeSet<_> = ["root.rs", "src/main.rs", "src/nested/module.rs"]
            .map(|path| fs::canonicalize(root.join(path)).unwrap())
            .into_iter()
            .collect();

        let mut files = BTreeSet::new();
        let mut directories = BTreeSet::new();
        collect(&root, &mut files, &mut directories).unwrap();
        collect(&root.join("src"), &mut files, &mut directories).unwrap();
        collect(&root.join("src/main.rs"), &mut files, &mut directories).unwrap();
        assert_eq!(files, expected);
        files.clear();
        directories.clear();
        collect(&root.join("src"), &mut files, &mut directories).unwrap();
        assert_eq!(files.len(), 2);
        files.clear();
        collect(&root.join("src/main.rs"), &mut files, &mut directories).unwrap();

        assert_eq!(
            files,
            BTreeSet::from([fs::canonicalize(root.join("src/main.rs")).unwrap()])
        );

        assert!(collect(&root.join("missing"), &mut files, &mut directories).is_err());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn accepts_default_directory_file_and_multiple_paths() {
        for (input, expected) in [
            (vec!["breathers"], vec!["."]),
            (vec!["breathers", "src/"], vec!["src/"]),
            (vec!["breathers", "src/main.rs"], vec!["src/main.rs"]),
            (
                vec!["breathers", "src/", "main.rs"],
                vec!["src/", "main.rs"],
            ),
        ] {
            let parsed = Arguments::try_parse_from(input).unwrap();

            assert_eq!(
                parsed.paths,
                expected.into_iter().map(PathBuf::from).collect::<Vec<_>>()
            );
        }
    }

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
    fn spaces_multiline_struct_fields() {
        let source = "struct Processor {\n    name: String,\n    enabled: bool,\n    callback: Box<\n        dyn Fn(&Request) -> Result<Response, Error>\n            + Send\n            + Sync,\n    >,\n    retries: usize,\n}\n";
        let expected = source
            .replace("    callback:", "\n    callback:")
            .replace("    retries:", "\n    retries:");
        assert_eq!(breathe(source).unwrap(), expected);
        assert_eq!(breathe(&expected).unwrap(), expected);
        assert_eq!(
            breathe(&source.replace('\n', "\r\n")).unwrap(),
            expected.replace('\n', "\r\n")
        );
        let source = "struct Tuple(\n    u32,\n    Box<\n        String,\n    >,\n    bool,\n);\n";
        let expected = source
            .replace("    Box<", "\n    Box<")
            .replace("    bool,", "\n    bool,");
        assert_eq!(breathe(source).unwrap(), expected);
        for source in [
            "struct Simple {\n    first: u32,\n    second: bool,\n}\n",
            "struct Only {\n    value: Box<\n        String,\n    >,\n}\n",
        ] {
            assert_eq!(breathe(source).unwrap(), source);
        }
    }

    #[test]
    fn spaces_returns_and_final_expressions() {
        for expression in [
            "return result;",
            "result",
            "Ok(result)",
            "return;",
            "return result",
        ] {
            let source = format!(
                "fn example() {{\n    let result = calculate();\n    save();\n    {expression}\n}}\n"
            );
            let expected = source.replace("    save();\n", "    save();\n\n");
            assert_eq!(breathe(&source).unwrap(), expected);
            assert_eq!(breathe(&expected).unwrap(), expected);
        }
        for source in [
            "fn example() {\n    return result;\n}\n",
            "fn example() {\n    result\n}\n",
            "fn example() { first(); result }\n",
            "fn example() {\n    first();\n    let callback = || return result;\n    finish();\n}\n",
        ] {
            assert_eq!(breathe(source).unwrap(), source);
        }
    }

    #[test]
    fn spaces_multiline_enum_variants() {
        let source = "enum Example {\n    First,\n    Second(u32), // trailing\n    // attached\n    Record {\n        value: u32,\n    },\n    Tuple(\n        u32,\n        String,\n    ),\n    Last,\n}\n";
        let expected = source
            .replace("// trailing\n", "// trailing\n\n")
            .replace("    },\n", "    },\n\n")
            .replace("    ),\n", "    ),\n\n");
        assert_eq!(breathe(source).unwrap(), expected);
        assert_eq!(breathe(&expected).unwrap(), expected);
        assert_eq!(
            breathe(&source.replace('\n', "\r\n")).unwrap(),
            expected.replace('\n', "\r\n")
        );
        for source in [
            "enum Example { First, Second(u32), Record { value: u32 } }\n",
            "enum Example {\n    First,\n    Second(u32),\n    Record { value: u32 },\n}\n",
            "enum Example {\n    Record {\n        value: u32,\n    },\n}\n",
        ] {
            assert_eq!(breathe(source).unwrap(), source);
        }
    }

    #[test]
    fn spaces_multiline_match_arms() {
        let source = "fn example() {\n    match kind {\n        Kind::First => 1,\n        Kind::Second => 2, // trailing\n        // attached\n        Kind::Index => Parts::Index {\n            receiver: children.next()?,\n            key: children.next()?,\n        },\n        Kind::Instantiate => Parts::Instantiate {\n            expression: children.next()?,\n            arguments: children.next()?,\n        },\n        Kind::Last => 3,\n    }\n}\n";
        let expected = source
            .replace("// trailing\n", "// trailing\n\n")
            .replace("        },\n", "        },\n\n");
        assert_eq!(breathe(source).unwrap(), expected);
        assert_eq!(breathe(&expected).unwrap(), expected);
        assert_eq!(
            breathe(&source.replace('\n', "\r\n")).unwrap(),
            expected.replace('\n', "\r\n")
        );
        for source in [
            "fn example() {\n    match kind {\n        First => if ready() { 1 } else { 2 },\n        Second => { 3 },\n        _ => 4,\n    }\n}\n",
            "fn example() {\n    match kind {\n        First => r#\"first\nsecond\"#,\n        _ => \"last\",\n    }\n}\n",
        ] {
            assert_eq!(breathe(source).unwrap(), source);
        }
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
