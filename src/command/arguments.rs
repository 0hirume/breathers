use std::path::PathBuf;

use clap::{Args, Parser, ValueEnum};

use crate::languages::Language;

#[derive(Parser)]
#[command(version, about = "Give source code breathing room")]
pub(super) struct Arguments {
    #[arg(
        default_value = ".",
        help = "Source files or directories to format recursively, or - for stdin"
    )]
    pub(super) paths: Vec<PathBuf>,

    #[arg(
        short,
        long,
        value_enum,
        help = "Override language detection; required for stdin without --stdin-filepath"
    )]
    pub(super) language: Option<Language>,

    #[arg(short, long, help = "Read configuration from this TOML file")]
    pub(super) config: Option<PathBuf>,

    #[command(flatten)]
    pub(super) formatting: Formatting,

    #[command(flatten)]
    pub(super) output: Output,

    #[arg(
        short = 'i',
        long,
        value_name = "PATTERN",
        help = "Select files matching a project-relative glob; repeatable, replaces configured includes"
    )]
    pub(super) include: Vec<String>,

    #[arg(
        short = 'e',
        long,
        value_name = "PATTERN",
        help = "Exclude a project-relative glob; repeatable, adds to configured exclusions"
    )]
    pub(super) exclude: Vec<String>,

    #[arg(
        short = 'f',
        long,
        value_name = "PATH",
        requires = "paths",
        help = "Use this filepath for stdin language, configuration, and compiler context; never write it"
    )]
    pub(super) stdin_filepath: Option<PathBuf>,

    #[arg(
        short = 'S',
        long,
        conflicts_with_all = ["check", "preview", "source", "quiet", "verbose", "include", "exclude", "stdin_filepath", "json"],
        help = "Print the configuration JSON Schema without formatting files"
    )]
    pub(super) schema: bool,
}

#[derive(Clone, Copy, Default, ValueEnum)]
pub(super) enum Color {
    #[default]
    Auto,

    Always,
    Never,
}

#[derive(Args)]
pub(super) struct Formatting {
    #[arg(short = 'k', long, conflicts_with_all = ["preview", "source"], help = "Check formatting without writing; exit with failure if changes are needed")]
    pub(super) check: bool,

    #[arg(
        short = 'p',
        long,
        conflicts_with = "source",
        help = "Print unified diffs without writing files"
    )]
    pub(super) preview: bool,

    #[arg(
        short = 's',
        long,
        conflicts_with = "json",
        help = "Print one formatted source to stdout without writing files"
    )]
    pub(super) source: bool,
}

#[derive(Args)]
pub(super) struct Output {
    #[arg(
        short = 'q',
        long,
        conflicts_with = "verbose",
        help = "Hide successful file reports and summary; keep errors visible"
    )]
    pub(super) quiet: bool,

    #[arg(short = 'v', long, help = "Show progress and unchanged files")]
    pub(super) verbose: bool,

    #[arg(short = 'j', long, help = "Print a JSON report to stdout")]
    pub(super) json: bool,

    #[arg(
        short = 'C',
        long,
        value_enum,
        default_value = "auto",
        help = "Control diagnostic colors"
    )]
    pub(super) color: Color,
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use clap::Parser as _;

    use crate::languages::Language;

    use super::Arguments;

    #[test]
    fn accepts_language_options() {
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
}
