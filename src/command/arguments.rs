use std::path::PathBuf;

use clap::Parser;

use crate::languages::Language;

#[derive(Parser)]
#[command(about = "Give source code breathing room")]
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
        help = "Override language detection; required for stdin"
    )]
    pub(super) language: Option<Language>,

    #[arg(short, long, help = "Read configuration from this TOML file")]
    pub(super) config: Option<PathBuf>,

    #[arg(
        long,
        help = "Print the configuration JSON Schema without formatting files"
    )]
    pub(super) schema: bool,

    #[arg(long = "lsp", conflicts_with_all = ["paths", "language", "schema"], help = "Run the formatting language server over stdio")]
    pub(super) server: bool,
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
