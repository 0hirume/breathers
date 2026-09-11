use std::{
    collections::BTreeSet,
    fs,
    io::{self, IsTerminal, Write},
    path::{Path, PathBuf},
    process::ExitCode,
    time::{Duration, Instant},
};

use clap::{Parser, ValueEnum};
use console::style;
use indicatif::{ProgressBar, ProgressStyle};

mod compilation;
mod configuration;
mod server;
mod spacing;
mod syntax;

mod languages {
    pub mod c;
    pub mod cplusplus;
    pub mod javascript;
    pub mod lua;
    pub mod luau;
    pub mod nushell;
    pub mod python;
    pub mod rust;
}

use languages::{c, cplusplus, javascript, lua, luau, nushell, python, rust};

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
enum Language {
    Rust,
    Lua,
    Luau,
    C,
    Python,
    Nushell,
    Javascript,
    Typescript,
    Tsx,

    #[value(name = "c++")]
    CPlusPlus,
}

impl Language {
    fn infer(path: &Path) -> Option<Self> {
        match path.extension()?.to_str()? {
            "rs" => Some(Self::Rust),
            "luau" => Some(Self::Luau),
            "lua" => Some(Self::Lua),
            "c" | "h" => Some(Self::C),
            "py" | "pyi" => Some(Self::Python),
            "nu" => Some(Self::Nushell),
            "js" | "jsx" | "mjs" | "cjs" => Some(Self::Javascript),
            "ts" | "mts" | "cts" => Some(Self::Typescript),
            "tsx" => Some(Self::Tsx),

            "C" | "H" | "cc" | "cpp" | "cxx" | "c++" | "hh" | "hpp" | "hxx" | "h++" => {
                Some(Self::CPlusPlus)
            }

            _ => None,
        }
    }

    fn breathe(
        self,
        source: &str,
        configuration: &configuration::Configuration,
    ) -> Result<String, String> {
        self.format(source, configuration, None)
    }

    fn format(
        self,
        source: &str,
        configuration: &configuration::Configuration,
        path: Option<&Path>,
    ) -> Result<String, String> {
        match self {
            Self::Rust => rust::breathe(source, &configuration.rust),
            Self::Luau => luau::breathe(source, &configuration.luau),
            Self::Lua => lua::breathe(source, &configuration.lua),
            Self::C => c::breathe(source, &tree_sitter_c::LANGUAGE.into(), &configuration.c),

            Self::CPlusPlus => cplusplus::breathe(source, path, &configuration.cplusplus),

            Self::Python => python::breathe(source, &configuration.python),
            Self::Nushell => nushell::breathe(source, &configuration.nushell),

            Self::Javascript => javascript::breathe(
                source,
                &tree_sitter_javascript::LANGUAGE.into(),
                &configuration.javascript,
            ),

            Self::Typescript => javascript::breathe(
                source,
                &tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
                &configuration.typescript,
            ),

            Self::Tsx => javascript::breathe(
                source,
                &tree_sitter_typescript::LANGUAGE_TSX.into(),
                &configuration.typescript,
            ),
        }
    }
}

#[derive(Parser)]
#[command(about = "Give source code breathing room")]
struct Arguments {
    #[arg(
        default_value = ".",
        help = "Source files or directories to format recursively, or - for stdin"
    )]
    paths: Vec<PathBuf>,

    #[arg(
        short,
        long,
        value_enum,
        help = "Override language detection; required for stdin"
    )]
    language: Option<Language>,

    #[arg(short, long, help = "Read configuration from this TOML file")]
    config: Option<PathBuf>,

    #[arg(
        long,
        help = "Print the configuration JSON Schema without formatting files"
    )]
    schema: bool,

    #[arg(long = "lsp", conflicts_with_all = ["paths", "language", "schema"], help = "Run the formatting language server over stdio")]
    server: bool,
}

fn collect(
    path: &Path,
    files: &mut BTreeSet<PathBuf>,
    directories: &mut BTreeSet<PathBuf>,
    selection: &configuration::Selection,
) -> Result<(), String> {
    let canonical =
        fs::canonicalize(path).map_err(|error| format!("{}: {error}", path.display()))?;

    let metadata =
        fs::metadata(&canonical).map_err(|error| format!("{}: {error}", path.display()))?;

    if metadata.is_dir() {
        if selection.excluded_directory(&canonical) || !directories.insert(canonical.clone()) {
            return Ok(());
        }

        for entry in
            fs::read_dir(&canonical).map_err(|error| format!("{}: {error}", path.display()))?
        {
            let entry = entry.map_err(|error| format!("{}: {error}", path.display()))?;

            let kind = entry
                .file_type()
                .map_err(|error| format!("{}: {error}", entry.path().display()))?;

            if kind.is_symlink() || (kind.is_dir() && entry.file_name() == ".git") {
                continue;
            }

            if kind.is_dir() || (kind.is_file() && Language::infer(&entry.path()).is_some()) {
                collect(&entry.path(), files, directories, selection)?;
            }
        }
    } else if metadata.is_file() {
        if selection.includes(&canonical) {
            files.insert(canonical);
        }
    } else {
        return Err(format!("{}: expected a file or directory", path.display()));
    }

    Ok(())
}

#[derive(Default)]
struct Summary {
    processed: usize,
    changed: usize,
    unchanged: usize,
    failed: usize,
}

fn report(error: &str) {
    eprintln!("{} {error}", style("error:").for_stderr().red().bold());
}

fn progress(progress: &ProgressBar, message: String) {
    if progress.is_hidden() {
        eprintln!("{message}");
    }

    progress.set_message(message);
}

fn process(
    files: BTreeSet<PathBuf>,
    language: Option<Language>,
    configuration: &configuration::Configuration,
    indicator: &ProgressBar,
    summary: &mut Summary,
    directory: &Path,
) -> Result<(), String> {
    let total = files.len();
    let mut changes = Vec::new();

    for path in files {
        let name = path
            .strip_prefix(directory)
            .unwrap_or(&path)
            .display()
            .to_string();

        summary.processed += 1;

        progress(
            indicator,
            format!("Checking {}/{total}  {name}", summary.processed),
        );

        let source = fs::read_to_string(&path).map_err(|error| {
            summary.failed += 1;

            format!("{name}\n  {error}")
        })?;

        let language = language.or_else(|| Language::infer(&path)).ok_or_else(|| {
            summary.failed += 1;

            format!("{name}\n  Unknown language; specify --language (-l)")
        })?;

        let output = match language.format(&source, configuration, Some(&path)) {
            Ok(output) => output,

            Err(error) => {
                summary.failed += 1;

                indicator.suspend(|| {
                    report(&format!(
                        "{name} (skipped)\n  {}",
                        error.replace('\n', "\n  ")
                    ));
                });

                continue;
            }
        };

        if source == output {
            summary.unchanged += 1;
            indicator.suspend(|| eprintln!("{}  {name}", style("Unchanged").for_stderr().dim()));
        } else {
            changes.push((path, name, output));
        }
    }

    let total = changes.len();

    for (path, name, output) in changes {
        progress(
            indicator,
            format!("Writing {}/{total}  {name}", summary.changed + 1),
        );

        fs::write(&path, output).map_err(|error| {
            summary.failed += 1;

            format!("{name}\n  {error}")
        })?;

        summary.changed += 1;
        indicator.suspend(|| eprintln!("{}  {name}", style("Formatted").for_stderr().green()));
    }

    Ok(())
}

fn batch(
    arguments: Arguments,
    configuration: &configuration::Configuration,
    selection: &configuration::Selection,
) -> Result<ExitCode, String> {
    let started = Instant::now();

    let directory = std::env::current_dir()
        .and_then(fs::canonicalize)
        .map_err(|error| error.to_string())?;

    let indicator = ProgressBar::new_spinner().with_style(
        ProgressStyle::with_template("{spinner:.cyan} {msg}").map_err(|error| error.to_string())?,
    );

    progress(&indicator, "Scanning files...".into());

    if !indicator.is_hidden() {
        indicator.enable_steady_tick(Duration::from_millis(120));
    }

    let mut summary = Summary::default();

    let result = (|| {
        let mut files = BTreeSet::new();
        let mut directories = BTreeSet::new();

        for path in arguments.paths {
            collect(&path, &mut files, &mut directories, selection)?;
        }

        process(
            files,
            arguments.language,
            configuration,
            &indicator,
            &mut summary,
            &directory,
        )
    })();

    indicator.finish_and_clear();

    if let Err(error) = &result {
        report(error);
        eprintln!("{}", style("Aborted").for_stderr().red());
    }

    let failures = style(summary.failed).for_stderr();

    let failures = if summary.failed == 0 {
        failures.dim()
    } else {
        failures.red()
    };

    eprintln!(
        "{} processed · {} changed · {} unchanged · {} failed · {:?} elapsed",
        summary.processed,
        style(summary.changed).for_stderr().green(),
        style(summary.unchanged).for_stderr().dim(),
        failures,
        style(started.elapsed()).for_stderr().cyan(),
    );

    let pending = summary.processed - summary.changed - summary.unchanged - summary.failed;

    if pending > 0 {
        eprintln!("{pending} prepared changes not written");
    }

    Ok(if result.is_ok() && summary.failed == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

fn run() -> Result<ExitCode, String> {
    let arguments = Arguments::parse();

    if arguments.server {
        return server::run(arguments.config).map(|()| ExitCode::SUCCESS);
    }

    if arguments.schema {
        println!(
            "{}",
            serde_json::to_string_pretty(&schemars::schema_for!(configuration::Configuration))
                .map_err(|error| error.to_string())?
        );

        return Ok(ExitCode::SUCCESS);
    }

    let input_language = if arguments.paths.iter().any(|path| path.as_os_str() == "-") {
        if arguments.paths.len() != 1 {
            return Err("Stdin (-) must be the only input path".into());
        }

        Some(arguments.language.ok_or("Stdin requires --language (-l)")?)
    } else {
        None
    };

    let (configuration, root) = configuration::Configuration::load(arguments.config.as_deref())?;
    let selection = configuration::Selection::new(&configuration, root)?;

    if let Some(language) = input_language {
        let source =
            io::read_to_string(io::stdin().lock()).map_err(|error| format!("stdin: {error}"))?;

        let formatted = language
            .breathe(&source, &configuration)
            .map_err(|error| format!("stdin: {error}"))?;

        let mut output = io::stdout().lock();

        return output
            .write_all(formatted.as_bytes())
            .and_then(|()| output.flush())
            .map(|()| ExitCode::SUCCESS)
            .map_err(|error| format!("stdout: {error}"));
    }

    batch(arguments, &configuration, &selection)
}

fn main() -> ExitCode {
    if !io::stderr().is_terminal() || std::env::var_os("NO_COLOR").is_some() {
        console::set_colors_enabled_stderr(false);
    }

    match run() {
        Ok(status) => status,

        Err(error) => {
            report(&error);

            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Arguments, Language, collect};
    fn breathe(source: &str) -> Result<String, String> {
        crate::languages::rust::breathe(source, &crate::configuration::Rules::default())
    }
    use clap::Parser;
    use std::{collections::BTreeSet, fs, path::PathBuf};

    #[test]
    fn selects_languages_and_accepts_shorthand() {
        for (path, expected) in [
            ("main.rs", Language::Rust),
            ("main.nu", Language::Nushell),
            ("main.py", Language::Python),
            ("main.pyi", Language::Python),
            ("main.js", Language::Javascript),
            ("main.jsx", Language::Javascript),
            ("main.mjs", Language::Javascript),
            ("main.cjs", Language::Javascript),
            ("main.ts", Language::Typescript),
            ("main.mts", Language::Typescript),
            ("main.cts", Language::Typescript),
            ("main.tsx", Language::Tsx),
            ("main.luau", Language::Luau),
            ("main.c", Language::C),
            ("header.h", Language::C),
            ("main.cpp", Language::CPlusPlus),
            ("main.cc", Language::CPlusPlus),
            ("main.cxx", Language::CPlusPlus),
            ("header.hpp", Language::CPlusPlus),
            ("header.hh", Language::CPlusPlus),
            ("header.hxx", Language::CPlusPlus),
            ("main.C", Language::CPlusPlus),
        ] {
            assert_eq!(Language::infer(std::path::Path::new(path)), Some(expected));
        }

        assert_eq!(Language::infer(std::path::Path::new("unknown")), None);

        assert_eq!(
            Language::infer(std::path::Path::new("main.lua")),
            Some(Language::Lua)
        );

        assert!(Arguments::try_parse_from(["breathers", "-l", "unknown"]).is_err());

        assert_eq!(
            Arguments::try_parse_from(["breathers"]).unwrap().language,
            None
        );

        for flag in ["-l", "--language"] {
            for (name, expected) in [
                ("rust", Language::Rust),
                ("c", Language::C),
                ("c++", Language::CPlusPlus),
                ("luau", Language::Luau),
                ("lua", Language::Lua),
            ] {
                assert_eq!(
                    Arguments::try_parse_from(["breathers", flag, name, "header.h"])
                        .unwrap()
                        .language,
                    Some(expected)
                );
            }
        }

        for (language, source, expected) in [
            (
                Language::Luau,
                "local value = 1\nreturn value\n",
                "local value = 1\n\nreturn value\n",
            ),
            (
                Language::Rust,
                "fn example() {\n    work();\n    result\n}\n",
                "fn example() {\n    work();\n\n    result\n}\n",
            ),
            (
                Language::C,
                "int example(void) {\n    work();\n    return 1;\n}\n",
                "int example(void) {\n    work();\n\n    return 1;\n}\n",
            ),
            (
                Language::CPlusPlus,
                "int example() {\n    auto value = 1;\n    return value;\n}\n",
                "int example() {\n    auto value = 1;\n\n    return value;\n}\n",
            ),
        ] {
            assert_eq!(
                language
                    .breathe(source, &crate::configuration::Configuration::default())
                    .unwrap(),
                expected
            );
        }
    }

    #[test]
    fn discovers_source_files_and_deduplicates_paths() {
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
            "src/main.c",
            "src/header.h",
            "src/main.cpp",
            "src/header.hpp",
            "src/notes.txt",
            "target/generated.rs",
            ".git/ignored.rs",
        ] {
            fs::write(root.join(file), "fn example() {}\n").unwrap();
        }

        let expected: BTreeSet<_> = [
            "root.rs",
            "src/main.rs",
            "src/nested/module.rs",
            "src/main.c",
            "src/header.h",
            "src/main.cpp",
            "src/header.hpp",
            "target/generated.rs",
        ]
        .map(|path| fs::canonicalize(root.join(path)).unwrap())
        .into_iter()
        .collect();

        let mut files = BTreeSet::new();
        let mut directories = BTreeSet::new();

        let selection = crate::configuration::Selection::new(
            &crate::configuration::Configuration::default(),
            root.clone(),
        )
        .unwrap();

        collect(&root, &mut files, &mut directories, &selection).unwrap();
        collect(&root.join("src"), &mut files, &mut directories, &selection).unwrap();

        collect(
            &root.join("src/main.rs"),
            &mut files,
            &mut directories,
            &selection,
        )
        .unwrap();

        assert_eq!(files, expected);
        files.clear();
        directories.clear();
        collect(&root.join("src"), &mut files, &mut directories, &selection).unwrap();
        assert_eq!(files.len(), 6);
        files.clear();

        collect(
            &root.join("src/main.rs"),
            &mut files,
            &mut directories,
            &selection,
        )
        .unwrap();

        assert_eq!(
            files,
            BTreeSet::from([fs::canonicalize(root.join("src/main.rs")).unwrap()])
        );

        assert!(
            collect(
                &root.join("missing"),
                &mut files,
                &mut directories,
                &selection
            )
            .is_err()
        );

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
