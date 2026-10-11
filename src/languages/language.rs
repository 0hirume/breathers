use std::path::Path;

use clap::ValueEnum;

use crate::configuration::Configuration;

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
        path: Option<&Path>,
    ) -> Result<String, String> {
        match self {
            Self::Rust => rust::breathe(source, &configuration.rust),
            Self::Luau => luau::breathe(source, &configuration.luau),
            Self::Lua => lua::breathe(source, &configuration.lua),
            Self::C => c::breathe(source, path, &configuration.c, c::Dialect::C),

            Self::CPlusPlus => c::breathe(
                source,
                path,
                &configuration.cplusplus,
                c::Dialect::CPlusPlus,
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

#[cfg(test)]
mod tests {
    use std::path::Path;

    use crate::configuration::Configuration;

    use super::Language;

    #[test]
    fn infers_file_languages() {
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
            assert_eq!(Language::infer(Path::new(path)), Some(expected));
        }

        assert_eq!(Language::infer(Path::new("unknown")), None);

        assert_eq!(Language::infer(Path::new("main.lua")), Some(Language::Lua));
    }

    #[test]
    fn dispatches_formatting() {
        for (language, source, expected) in [
            (
                Language::Luau,
                r"local value = 1
return value
",
                r"local value = 1

return value
",
            ),
            (
                Language::Rust,
                r"fn example() {
    work();
    result
}
",
                r"fn example() {
    work();

    result
}
",
            ),
            (
                Language::C,
                r"int example(void) {
    int value = 1;
    return value;
}
",
                r"int example(void) {
    int value = 1;

    return value;
}
",
            ),
            (
                Language::CPlusPlus,
                r"int example() {
    auto value = 1;
    return value;
}
",
                r"int example() {
    auto value = 1;

    return value;
}
",
            ),
        ] {
            assert_eq!(
                language
                    .format(source, &Configuration::default(), None)
                    .unwrap(),
                expected
            );
        }
    }
}
