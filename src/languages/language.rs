use std::path::Path;

use clap::ValueEnum;

use crate::{configuration::Configuration, error::Failure};

use super::{c, javascript, lua, luau, nushell, powershell, python, rust};

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
pub(crate) enum Language {
    Rust,
    Lua,
    Luau,
    C,
    Python,
    Nushell,
    Powershell,
    Javascript,
    Typescript,
    Tsx,

    #[value(name = "c++")]
    CPlusPlus,
}

impl Language {
    pub(crate) fn infer(path: &Path) -> Option<Self> {
        match path.extension()?.to_str()? {
            "rs" => Some(Self::Rust),
            "luau" => Some(Self::Luau),
            "lua" => Some(Self::Lua),
            "c" | "h" => Some(Self::C),
            "py" | "pyi" => Some(Self::Python),
            "nu" => Some(Self::Nushell),
            "ps1" | "psm1" | "psd1" => Some(Self::Powershell),
            "js" | "jsx" | "mjs" | "cjs" => Some(Self::Javascript),
            "ts" | "mts" | "cts" => Some(Self::Typescript),
            "tsx" => Some(Self::Tsx),

            "C" | "H" | "cc" | "cpp" | "cxx" | "c++" | "hh" | "hpp" | "hxx" | "h++" => {
                Some(Self::CPlusPlus)
            }

            _ => None,
        }
    }

    pub(crate) fn format(
        self,
        source: &str,
        configuration: &Configuration,
    ) -> Result<String, Failure> {
        match self {
            Self::Rust => rust::breathe(source, &configuration.rust),
            Self::Luau => luau::breathe(source, &configuration.luau),
            Self::Lua => lua::breathe(source, &configuration.lua),
            Self::C => c::breathe(source, &tree_sitter_c::LANGUAGE.into(), &configuration.c),

            Self::CPlusPlus => c::breathe(
                source,
                &tree_sitter_cpp::LANGUAGE.into(),
                &configuration.cplusplus,
            ),

            Self::Python => python::breathe(source, &configuration.python),
            Self::Nushell => nushell::breathe(source, &configuration.nushell),
            Self::Powershell => powershell::breathe(source, &configuration.powershell),

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
