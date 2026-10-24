use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::Rules;

use super::schema::{
    C, CPlusPlus, Javascript, Lua, Luau, Nushell, Powershell, Python, Rust, Typescript, document,
};

#[derive(Default, Deserialize, JsonSchema, Serialize)]
#[serde(default, deny_unknown_fields)]
#[schemars(description = "Source spacing configuration. Project keys override matching global keys; omitted keys inherit global settings, then built-in defaults.", transform = document)]
pub(crate) struct Configuration {
    #[schemars(
        description = "Select files using ordered Git-ignore-style patterns rooted at the project configuration directory, or the working directory when no project configuration exists. Bare names match at any depth; leading / anchors to the root; trailing / matches directories. Positive patterns select files and directory contents; ! removes them from selection. Default: inherit the global list, otherwise select all eligible files. An empty list selects none. Exclusions win."
    )]
    pub(crate) include: Option<Vec<String>>,

    #[schemars(
        description = "Exclude files and directories using ordered Git-ignore-style patterns, with the same root as include. Bare names match at any depth; leading / anchors to the root; trailing / matches directories. Later matching rules win; ! re-includes a path only when its parent directories are not excluded. Default: inherit the global list, otherwise add no exclusions. An empty list clears inherited exclusions; directory searches still skip .git, target, .venv, node_modules, and symbolic links."
    )]
    pub(crate) exclude: Option<Vec<String>>,

    #[serde(deserialize_with = "Rust::deserialize")]
    #[schemars(
        with = "BTreeMap<Rust, bool>",
        description = "Rust spacing rules. Default: inherit global settings, then built-in defaults."
    )]
    pub(crate) rust: Rules,

    #[serde(deserialize_with = "Lua::deserialize")]
    #[schemars(
        with = "BTreeMap<Lua, bool>",
        description = "Lua spacing rules. Default: inherit global settings, then built-in defaults."
    )]
    pub(crate) lua: Rules,

    #[serde(deserialize_with = "Luau::deserialize")]
    #[schemars(
        with = "BTreeMap<Luau, bool>",
        description = "Luau spacing and module-grouping rules. Default: inherit global settings, then built-in defaults."
    )]
    pub(crate) luau: Rules,

    #[serde(deserialize_with = "C::deserialize")]
    #[schemars(
        with = "BTreeMap<C, bool>",
        description = "C spacing rules. Default: inherit global settings, then built-in defaults."
    )]
    pub(crate) c: Rules,

    #[serde(rename = "c++", deserialize_with = "CPlusPlus::deserialize")]
    #[schemars(
        with = "BTreeMap<CPlusPlus, bool>",
        description = "C++ spacing rules. Default: inherit global settings, then built-in defaults."
    )]
    pub(crate) cplusplus: Rules,

    #[serde(deserialize_with = "Nushell::deserialize")]
    #[schemars(
        with = "BTreeMap<Nushell, bool>",
        description = "Nushell spacing rules. Default: inherit global settings, then built-in defaults."
    )]
    pub(crate) nushell: Rules,

    #[serde(deserialize_with = "Powershell::deserialize")]
    #[schemars(
        with = "BTreeMap<Powershell, bool>",
        description = "PowerShell spacing rules. Default: inherit global settings, then built-in defaults."
    )]
    pub(crate) powershell: Rules,

    #[serde(deserialize_with = "Python::deserialize")]
    #[schemars(
        with = "BTreeMap<Python, bool>",
        description = "Python spacing rules. Default: inherit global settings, then built-in defaults."
    )]
    pub(crate) python: Rules,

    #[serde(deserialize_with = "Javascript::deserialize")]
    #[schemars(
        with = "BTreeMap<Javascript, bool>",
        description = "JavaScript and JSX spacing rules. Default: inherit global settings, then built-in defaults."
    )]
    pub(crate) javascript: Rules,

    #[serde(deserialize_with = "Typescript::deserialize")]
    #[schemars(
        with = "BTreeMap<Typescript, bool>",
        description = "TypeScript and TSX spacing rules. Default: inherit global settings, then built-in defaults."
    )]
    pub(crate) typescript: Rules,
}

impl Configuration {
    pub(super) fn overlay(&mut self, project: Self) {
        if project.include.is_some() {
            self.include = project.include;
        }

        if project.exclude.is_some() {
            self.exclude = project.exclude;
        }

        self.rust.0.extend(project.rust.0);
        self.lua.0.extend(project.lua.0);
        self.luau.0.extend(project.luau.0);
        self.c.0.extend(project.c.0);
        self.cplusplus.0.extend(project.cplusplus.0);
        self.nushell.0.extend(project.nushell.0);
        self.powershell.0.extend(project.powershell.0);

        self.python.0.extend(project.python.0);
        self.javascript.0.extend(project.javascript.0);
        self.typescript.0.extend(project.typescript.0);
    }
}
