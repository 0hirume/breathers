use std::{
    collections::BTreeMap,
    fs,
    io::ErrorKind,
    path::{Path, PathBuf},
};

use globset::{Glob, GlobSet, GlobSetBuilder};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum Rule {
    Blocks,
    Multiline,
    Returns,
    Fields,
    Arms,
    Variants,
    Groups,
    Related,
    Conditionals,
    ForLoops,
    WhileLoops,
    RepeatLoops,
    LoopExpressions,
    DoBlocks,
    BlockExpressions,
    Matches,
    Switches,
    TryBlocks,
    WithBlocks,
    Functions,
    DecoratedFunctions,
    Classes,
    Interfaces,
    Calls,
    Arrays,
    Tables,
    Objects,
    Declarations,
    TypeAliases,
    ReturnStatements,
    TailExpressions,
    CoroutineReturns,
    StructFields,
    TupleFields,
    ClassMembers,
    InterfaceMembers,
    TypeMembers,
    ObjectProperties,
    DictionaryEntries,
    MatchArms,
    MatchCases,
    SwitchCases,
    EnumVariants,
    EnumMembers,
    Services,
    Requires,
    TypeGroups,
    ClassGroups,
    PopulatedDeclarations,
}

impl Rule {
    fn description(self) -> &'static str {
        match self {
            Self::Blocks => "Separate control flow and block declarations.",
            Self::Multiline => "Separate other multiline statements.",
            Self::Returns => "Separate noninitial returns and Rust tail expressions.",
            Self::Fields => "Separate multiline fields, members, and dictionary entries.",
            Self::Arms => "Separate multiline match arms and switch cases.",
            Self::Variants => "Separate multiline enum entries.",
            Self::Groups => "Separate Luau module groups.",

            Self::Related => {
                "Keep adjacent variable dependencies together without removing existing blank lines."
            }

            Self::Conditionals => "Separate conditional statements and expressions.",
            Self::ForLoops => "Separate for loops.",
            Self::WhileLoops => "Separate while loops.",
            Self::RepeatLoops => "Separate repeat-until loops.",
            Self::LoopExpressions => "Separate Rust loop expressions.",
            Self::DoBlocks => "Separate do blocks and do-while loops where supported.",
            Self::BlockExpressions => "Separate Rust block expressions.",
            Self::Matches => "Separate match statements and expressions.",
            Self::Switches => "Separate switch statements.",
            Self::TryBlocks => "Separate try statements.",
            Self::WithBlocks => "Separate with statements.",
            Self::Functions => "Separate function declarations.",
            Self::DecoratedFunctions => "Separate decorated Python functions.",
            Self::Classes => "Separate class declarations.",
            Self::Interfaces => "Separate TypeScript interface declarations.",
            Self::Calls => "Separate multiline call expressions.",
            Self::Arrays => "Separate multiline array and sequence expressions.",
            Self::Tables => "Separate multiline Lua and Luau table expressions.",
            Self::Objects => "Separate multiline object and dictionary expressions.",

            Self::Declarations => {
                "Separate multiline declarations not classified as calls, collections, or type aliases."
            }

            Self::TypeAliases => "Separate multiline type aliases.",
            Self::ReturnStatements => "Insert a blank line before noninitial return statements.",
            Self::TailExpressions => "Insert a blank line before noninitial Rust tail expressions.",

            Self::CoroutineReturns => {
                "Insert a blank line before noninitial C++ coroutine returns."
            }

            Self::StructFields => "Separate multiline struct fields.",
            Self::TupleFields => "Separate multiline Rust tuple-struct fields.",
            Self::ClassMembers => "Separate multiline class members.",
            Self::InterfaceMembers => "Separate multiline TypeScript interface members.",
            Self::TypeMembers => "Separate multiline TypeScript object-type members.",

            Self::ObjectProperties => {
                "Separate multiline JavaScript and TypeScript object properties."
            }

            Self::DictionaryEntries => "Separate multiline Python dictionary entries.",
            Self::MatchArms => "Separate multiline Rust match arms.",
            Self::MatchCases => "Separate multiline Python match cases.",
            Self::SwitchCases => "Separate multiline switch cases.",
            Self::EnumVariants => "Separate multiline Rust enum variants.",
            Self::EnumMembers => "Separate multiline C, C++, and TypeScript enum members.",
            Self::Services => "Separate Luau service groups from other module groups.",
            Self::Requires => "Separate Luau require groups by module path prefix.",
            Self::TypeGroups => "Separate imported, local, and exported Luau type groups.",
            Self::ClassGroups => "Separate Luau class groups from other module groups.",

            Self::PopulatedDeclarations => {
                "Separate Luau declarations populated by following field or index assignments."
            }
        }
    }

    fn parent(self) -> Option<Self> {
        match self {
            Self::Conditionals
            | Self::ForLoops
            | Self::WhileLoops
            | Self::RepeatLoops
            | Self::LoopExpressions
            | Self::DoBlocks
            | Self::BlockExpressions
            | Self::Matches
            | Self::Switches
            | Self::TryBlocks
            | Self::WithBlocks
            | Self::Functions
            | Self::DecoratedFunctions
            | Self::Classes
            | Self::Interfaces => Some(Self::Blocks),

            Self::Calls
            | Self::Arrays
            | Self::Tables
            | Self::Objects
            | Self::Declarations
            | Self::TypeAliases => Some(Self::Multiline),

            Self::ReturnStatements | Self::TailExpressions | Self::CoroutineReturns => {
                Some(Self::Returns)
            }

            Self::StructFields
            | Self::TupleFields
            | Self::ClassMembers
            | Self::InterfaceMembers
            | Self::TypeMembers
            | Self::ObjectProperties
            | Self::DictionaryEntries => Some(Self::Fields),

            Self::MatchArms | Self::MatchCases | Self::SwitchCases => Some(Self::Arms),
            Self::EnumVariants | Self::EnumMembers => Some(Self::Variants),

            Self::Services
            | Self::Requires
            | Self::TypeGroups
            | Self::ClassGroups
            | Self::PopulatedDeclarations => Some(Self::Groups),

            _ => None,
        }
    }
}

#[derive(Default, Deserialize, Serialize, JsonSchema)]
#[serde(transparent)]
pub struct Rules(BTreeMap<Rule, bool>);

impl Rules {
    pub fn enabled(&self, rule: Rule) -> bool {
        self.0.get(&rule).copied().unwrap_or_else(|| {
            rule != Rule::Related && rule.parent().is_none_or(|parent| self.enabled(parent))
        })
    }
}

macro_rules! language {
    ($name:ident, $deserialize:ident, $($rule:ident),+ $(,)?) => {
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize, JsonSchema)]
        #[serde(rename_all = "snake_case")]
        enum $name {
            Blocks, Multiline, Returns, Fields, Arms, Variants, Groups, Related, $($rule),+
        }
        fn $deserialize<'source, D: serde::Deserializer<'source>>(deserializer: D) -> Result<Rules, D::Error> {
            let values = BTreeMap::<$name, bool>::deserialize(deserializer)?;
            Ok(Rules(values.into_iter().map(|(key, value)| (match key {
                $name::Blocks => Rule::Blocks, $name::Multiline => Rule::Multiline, $name::Returns => Rule::Returns,
                $name::Fields => Rule::Fields, $name::Arms => Rule::Arms, $name::Variants => Rule::Variants,
                $name::Groups => Rule::Groups, $name::Related => Rule::Related, $($name::$rule => Rule::$rule),+
            }, value)).collect()))
        }
    };
}

language!(
    Rust,
    rust,
    Conditionals,
    ForLoops,
    WhileLoops,
    LoopExpressions,
    BlockExpressions,
    Matches,
    Functions,
    Calls,
    Arrays,
    Declarations,
    TypeAliases,
    ReturnStatements,
    TailExpressions,
    StructFields,
    TupleFields,
    MatchArms,
    EnumVariants
);
language!(
    Lua,
    lua,
    Conditionals,
    ForLoops,
    WhileLoops,
    RepeatLoops,
    DoBlocks,
    Functions,
    Calls,
    Tables,
    Declarations,
    ReturnStatements
);
language!(
    Luau,
    luau,
    Conditionals,
    ForLoops,
    WhileLoops,
    RepeatLoops,
    DoBlocks,
    Functions,
    Classes,
    Calls,
    Tables,
    Declarations,
    TypeAliases,
    ReturnStatements,
    Services,
    Requires,
    TypeGroups,
    ClassGroups,
    PopulatedDeclarations
);
language!(
    C,
    c,
    Conditionals,
    ForLoops,
    WhileLoops,
    DoBlocks,
    Switches,
    Functions,
    Calls,
    Arrays,
    Declarations,
    ReturnStatements,
    StructFields,
    SwitchCases,
    EnumMembers
);
language!(
    CPlusPlus,
    cplusplus,
    Conditionals,
    ForLoops,
    WhileLoops,
    DoBlocks,
    Switches,
    TryBlocks,
    Functions,
    Calls,
    Arrays,
    Declarations,
    ReturnStatements,
    CoroutineReturns,
    StructFields,
    ClassMembers,
    SwitchCases,
    EnumMembers
);
language!(
    Python,
    python,
    Conditionals,
    ForLoops,
    WhileLoops,
    Matches,
    TryBlocks,
    WithBlocks,
    Functions,
    DecoratedFunctions,
    Classes,
    Calls,
    Arrays,
    Objects,
    Declarations,
    TypeAliases,
    ReturnStatements,
    DictionaryEntries,
    MatchCases
);
language!(
    Javascript,
    javascript,
    Conditionals,
    ForLoops,
    WhileLoops,
    DoBlocks,
    Switches,
    TryBlocks,
    WithBlocks,
    Functions,
    Classes,
    Calls,
    Arrays,
    Objects,
    Declarations,
    ReturnStatements,
    ClassMembers,
    ObjectProperties,
    SwitchCases
);
language!(
    Typescript,
    typescript,
    Conditionals,
    ForLoops,
    WhileLoops,
    DoBlocks,
    Switches,
    TryBlocks,
    WithBlocks,
    Functions,
    Classes,
    Interfaces,
    Calls,
    Arrays,
    Objects,
    Declarations,
    TypeAliases,
    ReturnStatements,
    ClassMembers,
    ObjectProperties,
    InterfaceMembers,
    TypeMembers,
    SwitchCases,
    EnumMembers
);

#[derive(Default, Deserialize, Serialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
#[schemars(description = "Source spacing configuration. Project keys override matching global keys; omitted keys inherit global settings, then built-in defaults.", transform = document)]
pub struct Configuration {
    #[schemars(
        description = "Include files matching these globs, relative to the project configuration directory or current working directory. Default: inherit the global list, otherwise select all eligible files. An empty list selects none. Exclusions win."
    )]
    pub include: Option<Vec<String>>,

    #[schemars(
        description = "Exclude files and directories matching these globs, using the same root as include. Default: inherit the global list, otherwise add no exclusions. An empty list clears inherited exclusions; directory searches still skip .git and symbolic links."
    )]
    pub exclude: Option<Vec<String>>,

    #[serde(deserialize_with = "rust")]
    #[schemars(
        with = "BTreeMap<Rust, bool>",
        description = "Rust spacing rules. Default: inherit global settings, then built-in defaults."
    )]
    pub rust: Rules,

    #[serde(deserialize_with = "lua")]
    #[schemars(
        with = "BTreeMap<Lua, bool>",
        description = "Lua spacing rules. Default: inherit global settings, then built-in defaults."
    )]
    pub lua: Rules,

    #[serde(deserialize_with = "luau")]
    #[schemars(
        with = "BTreeMap<Luau, bool>",
        description = "Luau spacing and module-grouping rules. Default: inherit global settings, then built-in defaults."
    )]
    pub luau: Rules,

    #[serde(deserialize_with = "c")]
    #[schemars(
        with = "BTreeMap<C, bool>",
        description = "C spacing rules. Default: inherit global settings, then built-in defaults."
    )]
    pub c: Rules,

    #[serde(rename = "c++", deserialize_with = "cplusplus")]
    #[schemars(
        with = "BTreeMap<CPlusPlus, bool>",
        description = "C++ spacing rules. Default: inherit global settings, then built-in defaults."
    )]
    pub cplusplus: Rules,

    #[serde(deserialize_with = "python")]
    #[schemars(
        with = "BTreeMap<Python, bool>",
        description = "Python spacing rules. Default: inherit global settings, then built-in defaults."
    )]
    pub python: Rules,

    #[serde(deserialize_with = "javascript")]
    #[schemars(
        with = "BTreeMap<Javascript, bool>",
        description = "JavaScript and JSX spacing rules. Default: inherit global settings, then built-in defaults."
    )]
    pub javascript: Rules,

    #[serde(deserialize_with = "typescript")]
    #[schemars(
        with = "BTreeMap<Typescript, bool>",
        description = "TypeScript and TSX spacing rules. Default: inherit global settings, then built-in defaults."
    )]
    pub typescript: Rules,
}

fn document(schema: &mut schemars::Schema) {
    let properties = schema
        .get_mut("properties")
        .and_then(serde_json::Value::as_object_mut)
        .expect("Configuration properties");

    for (name, section) in properties {
        if matches!(name.as_str(), "include" | "exclude") {
            section
                .as_object_mut()
                .expect("Selection schema")
                .remove("default");
        }

        if let Some(properties) = section
            .get_mut("properties")
            .and_then(serde_json::Value::as_object_mut)
        {
            for (name, property) in properties {
                let rule: Rule =
                    serde_json::from_value(name.clone().into()).expect("Known configuration rule");

                let inheritance = if let Some(parent) = rule.parent() {
                    let parent = serde_json::to_value(parent).expect("Rule name");

                    format!(
                        " Inherits `{}`.",
                        parent.as_str().expect("Rule name string")
                    )
                } else {
                    let enabled = Rules::default().enabled(rule);
                    property["default"] = enabled.into();

                    String::new()
                };

                property["description"] = format!(
                    "{}{inheritance} Omitted project keys inherit global settings first.",
                    rule.description()
                )
                .into();
            }
        }
    }
}

impl Configuration {
    fn read(path: &Path) -> Result<Option<Self>, String> {
        match fs::read_to_string(path) {
            Ok(source) => toml::from_str(&source)
                .map(Some)
                .map_err(|error| format!("{}: {error}", path.display())),

            Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
            Err(error) => Err(format!("{}: {error}", path.display())),
        }
    }

    fn overlay(&mut self, project: Self) {
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
        self.python.0.extend(project.python.0);
        self.javascript.0.extend(project.javascript.0);
        self.typescript.0.extend(project.typescript.0);
    }

    fn load_from(
        directory: &Path,
        global: Option<&Path>,
        explicit: Option<&Path>,
    ) -> Result<(Self, PathBuf), String> {
        let mut configuration = global
            .map(Self::read)
            .transpose()?
            .flatten()
            .unwrap_or_default();

        let mut root = directory.to_path_buf();

        if let Some(path) = explicit {
            let path = directory.join(path);

            let project = Self::read(&path)?
                .ok_or_else(|| format!("{}: configuration not found", path.display()))?;

            configuration.overlay(project);

            root = path
                .parent()
                .ok_or("Configuration has no parent directory")?
                .to_path_buf();
        } else {
            for ancestor in directory.ancestors() {
                if let Some(project) = Self::read(&ancestor.join("breathers.toml"))? {
                    configuration.overlay(project);
                    root = ancestor.to_path_buf();
                    break;
                }
            }
        }

        let root =
            fs::canonicalize(&root).map_err(|error| format!("{}: {error}", root.display()))?;

        Ok((configuration, root))
    }

    pub fn load(path: Option<&Path>) -> Result<(Self, PathBuf), String> {
        let directory = std::env::current_dir().map_err(|error| error.to_string())?;
        let global = dirs::config_dir().map(|directory| directory.join("breathers/config.toml"));

        Self::load_from(&directory, global.as_deref(), path)
    }
}

pub struct Selection {
    root: PathBuf,
    include: Option<GlobSet>,
    exclude: GlobSet,
}

impl Selection {
    pub fn new(configuration: &Configuration, root: PathBuf) -> Result<Self, String> {
        fn compile(patterns: &[String]) -> Result<GlobSet, String> {
            let mut builder = GlobSetBuilder::new();

            for pattern in patterns {
                builder.add(Glob::new(pattern).map_err(|error| format!("{pattern}: {error}"))?);
            }

            builder.build().map_err(|error| error.to_string())
        }

        Ok(Self {
            root,
            include: configuration.include.as_deref().map(compile).transpose()?,
            exclude: compile(configuration.exclude.as_deref().unwrap_or_default())?,
        })
    }

    pub fn excluded(&self, path: &Path) -> bool {
        path.strip_prefix(&self.root)
            .unwrap_or(path)
            .ancestors()
            .any(|ancestor| self.exclude.is_match(ancestor))
    }

    pub fn excluded_directory(&self, path: &Path) -> bool {
        self.excluded(path)
            || self
                .exclude
                .is_match(path.strip_prefix(&self.root).unwrap_or(path).join(""))
    }

    pub fn includes(&self, path: &Path) -> bool {
        !self.excluded(path)
            && self.include.as_ref().is_none_or(|patterns| {
                patterns.is_match(path.strip_prefix(&self.root).unwrap_or(path))
            })
    }
}

#[cfg(test)]
mod tests {
    use super::{Configuration, Rule};

    use crate::Language;

    const OVERRIDES: &[(Language, &str, &str, &str, &str)] = &[
        (
            Language::Rust,
            "rust",
            "returns",
            "tail_expressions",
            "fn example() {\n    work();\n    result\n}\n",
        ),
        (
            Language::Rust,
            "rust",
            "returns",
            "return_statements",
            "fn example() {\n    work();\n    return result;\n}\n",
        ),
        (
            Language::Rust,
            "rust",
            "arms",
            "match_arms",
            "fn example() {\n    match value {\n        First => 1,\n        Second => {\n            2\n        },\n    }\n}\n",
        ),
        (
            Language::Rust,
            "rust",
            "variants",
            "enum_variants",
            "enum Example {\n    First,\n    Second {\n        value: u32,\n    },\n}\n",
        ),
        (
            Language::Rust,
            "rust",
            "multiline",
            "calls",
            "fn example() {\n    first();\n    process(\n        value,\n    );\n    last();\n}\n",
        ),
        (
            Language::Lua,
            "lua",
            "blocks",
            "for_loops",
            "work()\nfor index = 1, 3 do\n    process(index)\nend\nfinish()\n",
        ),
        (
            Language::Luau,
            "luau",
            "groups",
            "requires",
            "local first = require(\"@first/path\")\nlocal second = require(\"@second/path\")\n",
        ),
        (
            Language::Luau,
            "luau",
            "multiline",
            "type_aliases",
            "local first = 1\ntype Value = {\n    value: number,\n}\nlocal second = 2\n",
        ),
        (
            Language::C,
            "c",
            "blocks",
            "while_loops",
            "void example(void) {\n    work();\n    while (ready()) {\n        process();\n    }\n    finish();\n}\n",
        ),
        (
            Language::CPlusPlus,
            "c++",
            "arms",
            "switch_cases",
            "void example(int value) {\n    switch (value) {\n        case 1:\n            work();\n            break;\n        default:\n            finish();\n            break;\n    }\n}\n",
        ),
        (
            Language::Python,
            "python",
            "blocks",
            "decorated_functions",
            "work()\n@decorate\ndef example():\n    pass\nfinish()\n",
        ),
        (
            Language::Python,
            "python",
            "arms",
            "match_cases",
            "match value:\n    case 1:\n        work()\n    case _:\n        finish()\n",
        ),
        (
            Language::Javascript,
            "javascript",
            "fields",
            "object_properties",
            "const values = {\n    first: 1,\n    second: [\n        2,\n    ],\n};\n",
        ),
        (
            Language::Typescript,
            "typescript",
            "fields",
            "interface_members",
            "interface Example {\n    first: number;\n    second: {\n        value: number;\n    };\n}\n",
        ),
        (
            Language::Typescript,
            "typescript",
            "multiline",
            "type_aliases",
            "const first = 1;\ntype Example = {\n    value: number;\n};\nconst second = 2;\n",
        ),
        (
            Language::Tsx,
            "typescript",
            "returns",
            "return_statements",
            "function example() {\n    work();\n    return <div />;\n}\n",
        ),
    ];

    #[test]
    fn layers_global_and_nearest_project_settings() {
        use std::{
            fs,
            time::{SystemTime, UNIX_EPOCH},
        };

        let root = std::env::temp_dir()
            .join("breathers")
            .join(std::process::id().to_string())
            .join(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
                    .to_string(),
            );

        let project = root.join("project");
        let nested = project.join("nested");
        fs::create_dir_all(&nested).unwrap();
        let global = root.join("global.toml");
        fs::write(&global, "include = [\"**/*.rs\"]\nexclude = [\"vendor/**\"]\n[rust]\nreturns = false\ntail_expressions = true\nrelated = true\n[python]\nblocks = false\n").unwrap();

        fs::write(
            project.join("breathers.toml"),
            "[rust]\nreturns = true\nrelated = false\n[python]\nconditionals = true\n",
        )
        .unwrap();

        let (configuration, selected) =
            Configuration::load_from(&nested, Some(&global), None).unwrap();

        assert_eq!(selected, fs::canonicalize(&project).unwrap());
        assert_eq!(configuration.include.unwrap(), ["**/*.rs"]);
        assert_eq!(configuration.exclude.unwrap(), ["vendor/**"]);
        assert!(configuration.rust.enabled(Rule::ReturnStatements));
        assert!(configuration.rust.enabled(Rule::TailExpressions));
        assert!(!configuration.rust.enabled(Rule::Related));
        assert!(!configuration.python.enabled(Rule::Functions));
        assert!(configuration.python.enabled(Rule::Conditionals));

        fs::write(
            nested.join("breathers.toml"),
            "include = []\nexclude = []\n[rust]\nfields = false\n",
        )
        .unwrap();

        let (configuration, selected) =
            Configuration::load_from(&nested, Some(&global), None).unwrap();

        assert_eq!(selected, fs::canonicalize(&nested).unwrap());
        assert_eq!(configuration.include.unwrap(), Vec::<String>::new());
        assert_eq!(configuration.exclude.unwrap(), Vec::<String>::new());
        assert!(!configuration.rust.enabled(Rule::ReturnStatements));
        assert!(configuration.rust.enabled(Rule::Related));
        let explicit = root.join("selected.toml");
        fs::write(&explicit, "[rust]\nreturns = true\n").unwrap();

        let (configuration, selected) =
            Configuration::load_from(&nested, Some(&global), Some(&explicit)).unwrap();

        assert_eq!(selected, fs::canonicalize(&root).unwrap());
        assert!(configuration.rust.enabled(Rule::ReturnStatements));
        assert!(configuration.rust.enabled(Rule::StructFields));
        assert!(!configuration.python.enabled(Rule::Functions));

        assert!(
            Configuration::load_from(&nested, Some(&global), Some(&root.join("missing.toml")))
                .is_err()
        );

        fs::write(&global, "[rust]\nreturns = 1\n").unwrap();
        assert!(Configuration::load_from(&nested, Some(&global), Some(&explicit)).is_err());
        fs::remove_file(&global).unwrap();

        assert!(
            Configuration::load_from(&nested, Some(&global), Some(&explicit))
                .unwrap()
                .0
                .python
                .enabled(Rule::Functions)
        );

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn filters_relative_paths_with_exclusion_precedence() {
        use std::path::PathBuf;
        let root = PathBuf::from("workspace");

        let configuration: Configuration = toml::from_str(
            "include = [\"src/**/*.{rs,luau}\"]\nexclude = [\"**/generated/**\", \"src/skip.rs\"]",
        )
        .unwrap();

        let selection = super::Selection::new(&configuration, root.clone()).unwrap();
        assert!(selection.includes(&root.join("src/main.rs")));
        assert!(selection.includes(&root.join("src/nested/main.luau")));
        assert!(!selection.includes(&root.join("src/main.lua")));
        assert!(!selection.includes(&root.join("other/main.rs")));
        assert!(!selection.includes(&root.join("src/skip.rs")));
        assert!(!selection.includes(&root.join("src/generated/main.rs")));
        assert!(selection.excluded_directory(&root.join("src/generated")));

        for (text, expected) in [
            ("", true),
            ("include = []", false),
            ("exclude = [\"src\"]", false),
        ] {
            let configuration = toml::from_str(text).unwrap();

            assert_eq!(
                super::Selection::new(&configuration, root.clone())
                    .unwrap()
                    .includes(&root.join("src/main.rs")),
                expected
            );
        }

        for text in ["include = [\"[\"]", "exclude = [\"[\"]"] {
            assert!(super::Selection::new(&toml::from_str(text).unwrap(), root.clone()).is_err());
        }
    }

    const RELATED: &[(Language, &str, &str, &str)] = &[
        (
            Language::Python,
            "python",
            "values = []\nresult = [\n    value for value in values\n]\n",
            "result =",
        ),
        (
            Language::Python,
            "python",
            "value = build()\nprocess(\n    f'{value}'\n)\n",
            "process(",
        ),
        (
            Language::Python,
            "python",
            "first = second = build()\nif second:\n    work()\n",
            "if second",
        ),
        (
            Language::Rust,
            "rust",
            "fn example() {\n    let value = 1;\n    let object = Example {\n        value,\n    };\n}\n",
            "    let object",
        ),
        (
            Language::C,
            "c",
            "void example(void) {\n    value = build(\n        input\n    );\n    value += 1;\n}\n",
            "    value +=",
        ),
        (
            Language::Luau,
            "luau",
            "local values = {}\nfor key, value in values do\n    process(key, value)\nend\n",
            "for key",
        ),
        (
            Language::Lua,
            "lua",
            "local values = {}\nfor key, value in pairs(values) do\n    process(key, value)\nend\n",
            "for key",
        ),
        (
            Language::Luau,
            "luau",
            "local ready = check()\nif ready then\n    work()\nend\n",
            "if ready",
        ),
        (
            Language::Luau,
            "luau",
            "local first, second = build()\nprocess(\n    second\n)\n",
            "process(",
        ),
        (
            Language::Luau,
            "luau",
            "value = build()\nreturn value\n",
            "return value",
        ),
        (
            Language::Luau,
            "luau",
            "local values = {\n    first = 1,\n}\nvalues.second = 2\n",
            "values.second",
        ),
        (
            Language::Rust,
            "rust",
            "fn example() {\n    let values = build();\n    for value in values {\n        process(value);\n    }\n}\n",
            "    for value",
        ),
        (
            Language::Rust,
            "rust",
            "fn example() {\n    value = build();\n    return value;\n}\n",
            "    return value",
        ),
        (
            Language::Rust,
            "rust",
            "fn example() {\n    let (first, second) = build();\n    second\n}\n",
            "    second",
        ),
        (
            Language::C,
            "c",
            "void example(void) {\n    int ready = check();\n    if (ready) {\n        work();\n    }\n}\n",
            "    if (ready)",
        ),
        (
            Language::CPlusPlus,
            "c++",
            "void example() {\n    auto values = build();\n    for (auto value : values) {\n        process(value);\n    }\n}\n",
            "    for (auto",
        ),
        (
            Language::Python,
            "python",
            "values = []\nfor value in values:\n    process(value)\n",
            "for value",
        ),
        (
            Language::Python,
            "python",
            "def example():\n    value = build()\n    return value\n",
            "    return value",
        ),
        (
            Language::Javascript,
            "javascript",
            "const values = [];\nfor (const value of values) {\n    process(value);\n}\n",
            "for (const",
        ),
        (
            Language::Typescript,
            "typescript",
            "const value: number = build();\nprocess(\n    value,\n);\n",
            "process(",
        ),
        (
            Language::Tsx,
            "typescript",
            "function example() {\n    const value = build();\n    return <div>{value}</div>;\n}\n",
            "    return",
        ),
    ];

    #[test]
    fn keeps_adjacent_dependencies_together_without_removing_blank_lines() {
        for &(language, section, source, next) in RELATED {
            let configuration: Configuration =
                toml::from_str(&format!("[\"{section}\"]\nrelated = true\n")).unwrap();

            let baseline = language.breathe(source, &Configuration::default()).unwrap();
            let boundary = format!("\n\n{next}");
            assert!(baseline.contains(&boundary), "{section}: {source}");
            let expected = baseline.replacen(&boundary, &format!("\n{next}"), 1);

            assert_eq!(
                language.breathe(source, &configuration).unwrap(),
                expected,
                "{section}: {source}"
            );

            assert_eq!(
                language.breathe(&expected, &configuration).unwrap(),
                expected
            );

            assert_eq!(
                language.breathe(&baseline, &configuration).unwrap(),
                baseline
            );

            assert_eq!(
                language
                    .breathe(&source.replace('\n', "\r\n"), &configuration)
                    .unwrap(),
                expected.replace('\n', "\r\n")
            );
        }
    }

    const UNRELATED: &[(Language, &str, &str)] = &[
        (
            Language::Rust,
            "rust",
            "fn example() {\n    let value = 1;\n    (value) = [\n        2,\n    ];\n}\n",
        ),
        (
            Language::Luau,
            "luau",
            "local ready = true\nif object.ready then\n    work()\nend\n",
        ),
        (
            Language::Luau,
            "luau",
            "local ready = true\nif other then\n    use(ready)\nend\n",
        ),
        (
            Language::Luau,
            "luau",
            "local ready = true\nlocal function example()\n    return ready\nend\n",
        ),
        (
            Language::Luau,
            "luau",
            "local ready = true\nprocess(\n    { ready = false }\n)\n",
        ),
        (
            Language::Lua,
            "lua",
            "local ready = true\nif object.ready then\n    work()\nend\n",
        ),
        (
            Language::Lua,
            "lua",
            "local ready = true\nprocess(\n    { ready = false }\n)\n",
        ),
        (
            Language::Rust,
            "rust",
            "fn example() {\n    let ready = true;\n    if object.ready {\n        work();\n    }\n}\n",
        ),
        (
            Language::Rust,
            "rust",
            "fn example() {\n    let ready = true;\n    if other {\n        use_value(ready);\n    }\n}\n",
        ),
        (
            Language::Rust,
            "rust",
            "fn example() {\n    let value = 1;\n    value = [\n        2,\n    ];\n}\n",
        ),
        (
            Language::C,
            "c",
            "void example(void) {\n    int ready = 1;\n    if (object->ready) {\n        work();\n    }\n}\n",
        ),
        (
            Language::C,
            "c",
            "void example(void) {\n    int index = 1;\n    for (int index = 0; index < 10; index++) {\n        work();\n    }\n}\n",
        ),
        (
            Language::CPlusPlus,
            "c++",
            "void example() {\n    bool ready = true;\n    if (other) {\n        use_value(ready);\n    }\n}\n",
        ),
        (
            Language::Python,
            "python",
            "ready = True\nif object.ready:\n    work()\n",
        ),
        (
            Language::Python,
            "python",
            "ready = True\nif other:\n    use(ready)\n",
        ),
        (
            Language::Python,
            "python",
            "ready = True\nif text == 'ready':\n    work()\n",
        ),
        (
            Language::Python,
            "python",
            "value = 1\nresult = [\n    value for value in other\n]\n",
        ),
        (
            Language::Javascript,
            "javascript",
            "const ready = true;\nif (object.ready) {\n    work();\n}\n",
        ),
        (
            Language::Javascript,
            "javascript",
            "let value = 1;\nvalue = [\n    2,\n];\n",
        ),
        (
            Language::Typescript,
            "typescript",
            "const ready = true;\nif (ready_again) {\n    work();\n}\n",
        ),
        (
            Language::Tsx,
            "typescript",
            "function example() {\n    const div = 1;\n    return <div />;\n}\n",
        ),
    ];

    #[test]
    fn related_keeps_dependency_chains_together() {
        let source = "local first = build(\n    input\n)\nlocal second = first\nreturn second\n";
        let configuration = toml::from_str("[luau]\nrelated = true\n").unwrap();

        assert_eq!(
            Language::Luau.breathe(source, &configuration).unwrap(),
            source
        );

        assert!(!Configuration::default().luau.enabled(Rule::Related));

        assert_ne!(
            Language::Luau
                .breathe(source, &Configuration::default())
                .unwrap(),
            source
        );
    }

    #[test]
    fn related_ignores_properties_literals_writes_and_nested_scopes() {
        for &(language, section, source) in UNRELATED {
            let configuration: Configuration =
                toml::from_str(&format!("[\"{section}\"]\nrelated = true\n")).unwrap();

            let expected = language.breathe(source, &Configuration::default()).unwrap();
            assert_ne!(expected, source, "{section}: {source}");

            assert_eq!(
                language.breathe(source, &configuration).unwrap(),
                expected,
                "{section}: {source}"
            );
        }
    }

    #[test]
    fn granular_overrides_take_precedence_in_both_directions() {
        for &(language, section, group, specific, source) in OVERRIDES {
            let formatted = language.breathe(source, &Configuration::default()).unwrap();
            assert_ne!(formatted, source, "{section}.{specific}");

            for (group_value, override_value, expected) in [
                (false, None, source),
                (false, Some(true), formatted.as_str()),
                (true, Some(false), source),
            ] {
                let override_text = override_value
                    .map(|value| format!("{specific} = {value}\n"))
                    .unwrap_or_default();

                let text = format!("[\"{section}\"]\n{group} = {group_value}\n{override_text}");
                let configuration: Configuration = toml::from_str(&text).unwrap();

                assert_eq!(
                    language.breathe(source, &configuration).unwrap(),
                    expected,
                    "{text}"
                );
            }
        }
    }

    #[test]
    fn granular_keys_are_language_specific_and_preserve_group_defaults() {
        for source in [
            "[python]\nmatch_arms = false",
            "[lua]\ninterface_members = true",
            "[javascript]\ntype_aliases = false",
            "[rust]\ndecorated_functions = true",
        ] {
            assert!(toml::from_str::<Configuration>(source).is_err());
        }

        let configuration: Configuration =
            toml::from_str("[rust]\nreturns = false\ntail_expressions = true\n").unwrap();

        assert!(!configuration.rust.enabled(Rule::ReturnStatements));
        assert!(configuration.rust.enabled(Rule::TailExpressions));
        assert!(configuration.rust.enabled(Rule::Blocks));
        assert!(configuration.python.enabled(Rule::ReturnStatements));
    }

    #[test]
    fn schema_documents_every_option_and_its_actual_default() {
        let schema = schemars::schema_for!(Configuration);

        assert!(
            schema
                .get("description")
                .and_then(serde_json::Value::as_str)
                .is_some()
        );

        let properties = schema.get("properties").unwrap().as_object().unwrap();

        for (name, section) in properties {
            let description = section["description"].as_str().unwrap();
            assert!(description.contains("Default:"), "{name}");

            if matches!(name.as_str(), "include" | "exclude") {
                assert!(section.get("default").is_none());
                assert!(description.contains("inherit the global list"));
                continue;
            }

            for (name, property) in section["properties"].as_object().unwrap() {
                let rule: Rule = serde_json::from_value(name.clone().into()).unwrap();
                let description = property["description"].as_str().unwrap();
                assert!(description.starts_with(rule.description()), "{name}");
                assert!(!description.contains("Default:"), "{name}");

                if let Some(parent) = rule.parent() {
                    let parent = serde_json::to_value(parent).unwrap();

                    assert!(
                        description.contains(&format!("Inherits `{}`.", parent.as_str().unwrap())),
                        "{name}"
                    );

                    assert!(property.get("default").is_none(), "{name}");
                } else {
                    let enabled = super::Rules::default().enabled(rule);
                    assert_eq!(property["default"], enabled, "{name}");
                }
            }
        }
    }

    #[test]
    fn committed_schema_matches_the_configuration() {
        let stored: serde_json::Value =
            serde_json::from_str(include_str!("../schema.json")).unwrap();

        let generated = serde_json::to_value(schemars::schema_for!(Configuration)).unwrap();
        assert_eq!(stored, generated);
    }

    #[test]
    fn disables_each_rule_without_removing_existing_blank_lines() {
        use crate::Language;

        for (language, section, rule, source) in [
            (
                Language::Rust,
                "rust",
                "blocks",
                "fn example() {\n    first();\n    if ready() {}\n    last();\n}\n",
            ),
            (
                Language::Rust,
                "rust",
                "multiline",
                "fn example() {\n    first();\n    let value = call(\n        argument,\n    );\n    last();\n}\n",
            ),
            (
                Language::Rust,
                "rust",
                "returns",
                "fn example() {\n    first();\n    value\n}\n",
            ),
            (
                Language::Rust,
                "rust",
                "fields",
                "struct Example {\n    first: u32,\n    second: Box<\n        String,\n    >,\n}\n",
            ),
            (
                Language::Rust,
                "rust",
                "variants",
                "enum Example {\n    First,\n    Second {\n        value: u32,\n    },\n}\n",
            ),
            (
                Language::Rust,
                "rust",
                "arms",
                "fn example() {\n    match value {\n        First => 1,\n        Second => {\n            2\n        },\n    }\n}\n",
            ),
            (
                Language::Luau,
                "luau",
                "groups",
                "local players = game:GetService(\"Players\")\nlocal module = require(\"@module/path\")\n",
            ),
            (Language::Lua, "lua", "returns", "work()\nreturn 1\n"),
            (Language::Luau, "luau", "returns", "work()\nreturn 1\n"),
            (
                Language::C,
                "c",
                "returns",
                "int example(void) {\n    work();\n    return 1;\n}\n",
            ),
            (
                Language::CPlusPlus,
                "c++",
                "returns",
                "int example() {\n    work();\n    return 1;\n}\n",
            ),
            (
                Language::Python,
                "python",
                "returns",
                "def example():\n    work()\n    return 1\n",
            ),
            (
                Language::Javascript,
                "javascript",
                "returns",
                "function example() {\n    work();\n    return 1;\n}\n",
            ),
            (
                Language::Typescript,
                "typescript",
                "returns",
                "function example(): number {\n    work();\n    return 1;\n}\n",
            ),
            (
                Language::Tsx,
                "typescript",
                "returns",
                "function example() {\n    work();\n    return <div />;\n}\n",
            ),
        ] {
            let formatted = language.breathe(source, &Configuration::default()).unwrap();
            assert_ne!(formatted, source, "{section}.{rule}");

            let configuration: Configuration =
                toml::from_str(&format!("[\"{section}\"]\n{rule} = false\n")).unwrap();

            assert_eq!(
                language.breathe(source, &configuration).unwrap(),
                source,
                "{section}.{rule}"
            );

            assert_eq!(
                language.breathe(&formatted, &configuration).unwrap(),
                formatted
            );
        }
    }

    #[test]
    fn controls_fields_arms_and_variants_in_new_languages() {
        use crate::Language;

        for (language, section, rule, source) in [
            (
                Language::Python,
                "python",
                "fields",
                "values = {\n    'first': 1,\n    'second': [\n        2,\n    ],\n}\n",
            ),
            (
                Language::Python,
                "python",
                "arms",
                "match value:\n    case 1:\n        work()\n    case _:\n        finish()\n",
            ),
            (
                Language::Javascript,
                "javascript",
                "fields",
                "const values = {\n    first: 1,\n    second: [\n        2,\n    ],\n};\n",
            ),
            (
                Language::Javascript,
                "javascript",
                "arms",
                "switch (value) {\n    case 1:\n        work();\n        break;\n    default:\n        finish();\n        break;\n}\n",
            ),
            (
                Language::Typescript,
                "typescript",
                "fields",
                "interface Example {\n    first: number;\n    second: {\n        value: number;\n    };\n}\n",
            ),
            (
                Language::Typescript,
                "typescript",
                "variants",
                "enum Example {\n    First,\n    Second = calculate(\n        1,\n    ),\n}\n",
            ),
        ] {
            assert_ne!(
                language.breathe(source, &Configuration::default()).unwrap(),
                source
            );

            let configuration: Configuration =
                toml::from_str(&format!("[{section}]\n{rule} = false")).unwrap();

            assert_eq!(language.breathe(source, &configuration).unwrap(), source);
        }
    }

    #[test]
    fn resolves_partial_toggles_and_rejects_invalid_configuration() {
        let configuration: Configuration =
            toml::from_str("[rust]\nreturns = false\n[python]\nblocks = false\n").unwrap();

        assert!(!configuration.rust.enabled(Rule::Returns));
        assert!(configuration.rust.enabled(Rule::Blocks));
        assert!(!configuration.python.enabled(Rule::Blocks));
        assert!(configuration.javascript.enabled(Rule::Returns));

        for source in [
            "[rust]\nunknown = false",
            "[unknown]\nreturns = false",
            "[rust]\nreturns = 1",
            "[rust]\nreturns = 'false'",
        ] {
            assert!(toml::from_str::<Configuration>(source).is_err());
        }
    }
}
