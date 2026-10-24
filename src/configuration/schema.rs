use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{Rule, Rules};

macro_rules! language {
    ($name:ident, $($rule:ident),+ $(,)?) => {
        #[derive(Copy, Clone, Eq, Ord, PartialEq, PartialOrd, Deserialize, JsonSchema, Serialize)]
        #[serde(rename_all = "snake_case")]
        pub(super) enum $name {
            Blocks, Multiline, Returns, Fields, Arms, Variants, Groups, Related, $($rule),+
        }
        impl $name {
            pub(super) fn deserialize<'source, Deserializer: serde::Deserializer<'source>>(
                deserializer: Deserializer,
            ) -> Result<Rules, Deserializer::Error> {
                let values = BTreeMap::<Self, bool>::deserialize(deserializer)?;
                Ok(Rules(values.into_iter().map(|(key, value)| (match key {
                    Self::Blocks => Rule::Blocks, Self::Multiline => Rule::Multiline, Self::Returns => Rule::Returns,
                    Self::Fields => Rule::Fields, Self::Arms => Rule::Arms, Self::Variants => Rule::Variants,
                    Self::Groups => Rule::Groups, Self::Related => Rule::Related, $(Self::$rule => Rule::$rule),+
                }, value)).collect()))
            }
        }
    };
}

language!(
    Rust,
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
    Nushell,
    Conditionals,
    ForLoops,
    WhileLoops,
    LoopStatements,
    Matches,
    TryBlocks,
    Functions,
    Modules,
    Calls,
    Pipelines,
    Arrays,
    Tables,
    Objects,
    Declarations,
    ReturnStatements,
    RecordFields,
    MatchArms
);

language!(
    Powershell,
    Conditionals,
    ForLoops,
    WhileLoops,
    DoBlocks,
    Switches,
    TryBlocks,
    Functions,
    Classes,
    Calls,
    Pipelines,
    Arrays,
    Objects,
    Declarations,
    ReturnStatements,
    ClassMembers,
    DictionaryEntries,
    SwitchCases,
    EnumMembers
);

language!(
    Python,
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

pub(super) fn document(schema: &mut schemars::Schema) {
    let properties = schema
        .get_mut("properties")
        .and_then(serde_json::Value::as_object_mut)
        .expect("Configuration properties");

    for (language, section) in properties {
        if matches!(language.as_str(), "include" | "exclude") {
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

                let default = if matches!(
                    (language.as_str(), rule),
                    ("nushell", Rule::MatchArms | Rule::RecordFields)
                        | ("python", Rule::DictionaryEntries)
                ) {
                    property["default"] = false.into();

                    " Defaults to false."
                } else {
                    ""
                };

                property["description"] = format!(
                    "{}{inheritance}{default} Omitted project keys inherit global settings first.",
                    description(rule)
                )
                .into();
            }
        }
    }
}

fn description(rule: Rule) -> &'static str {
    match rule {
        Rule::Blocks => "Separate control flow and block declarations.",
        Rule::Multiline => "Separate other multiline statements.",
        Rule::Returns => "Separate noninitial returns and Rust tail expressions.",
        Rule::Fields => "Separate multiline fields, members, and dictionary entries.",
        Rule::Arms => "Separate multiline match arms and switch cases.",
        Rule::Variants => "Separate multiline enum entries.",
        Rule::Groups => "Separate Luau module groups.",

        Rule::Related => {
            "Keep adjacent variable dependencies together without removing existing blank lines."
        }

        Rule::Conditionals => "Separate conditional statements and expressions.",
        Rule::ForLoops => "Separate for loops.",
        Rule::WhileLoops => "Separate while loops.",
        Rule::RepeatLoops => "Separate repeat-until loops.",
        Rule::LoopExpressions => "Separate Rust loop expressions.",
        Rule::LoopStatements => "Separate Nushell loop statements.",
        Rule::Modules => "Separate Nushell module and export-env declarations.",
        Rule::DoBlocks => "Separate do blocks and do-while loops where supported.",
        Rule::BlockExpressions => "Separate Rust block expressions.",
        Rule::Matches => "Separate match statements and expressions.",
        Rule::Switches => "Separate switch statements.",
        Rule::TryBlocks => "Separate try statements.",
        Rule::WithBlocks => "Separate with statements.",
        Rule::Functions => "Separate function declarations.",
        Rule::DecoratedFunctions => "Separate decorated Python functions.",
        Rule::Classes => "Separate class declarations.",
        Rule::Interfaces => "Separate TypeScript interface declarations.",
        Rule::Calls => "Separate multiline call expressions.",
        Rule::Pipelines => "Separate multiline pipelines.",
        Rule::Arrays => "Separate multiline array and sequence expressions.",
        Rule::Tables => "Separate multiline Lua, Luau, and Nushell table expressions.",
        Rule::Objects => "Separate multiline object, dictionary, and record expressions.",

        Rule::Declarations => {
            "Separate multiline declarations not classified as calls, collections, or type aliases."
        }

        Rule::TypeAliases => "Separate multiline type aliases.",
        Rule::ReturnStatements => "Insert a blank line before noninitial return statements.",
        Rule::TailExpressions => "Insert a blank line before noninitial Rust tail expressions.",

        Rule::CoroutineReturns => "Insert a blank line before noninitial C++ coroutine returns.",

        Rule::StructFields => "Separate multiline struct fields.",
        Rule::TupleFields => "Separate multiline Rust tuple-struct fields.",
        Rule::ClassMembers => "Separate multiline class members.",
        Rule::InterfaceMembers => "Separate multiline TypeScript interface members.",
        Rule::TypeMembers => "Separate multiline TypeScript object-type members.",

        Rule::ObjectProperties => "Separate multiline JavaScript and TypeScript object properties.",

        Rule::DictionaryEntries => "Separate multiline Python and PowerShell dictionary entries.",

        Rule::RecordFields => "Separate multiline Nushell record fields.",
        Rule::MatchArms => "Separate multiline Rust and Nushell match arms.",
        Rule::MatchCases => "Separate multiline Python match cases.",
        Rule::SwitchCases => "Separate multiline switch cases.",
        Rule::EnumVariants => "Separate multiline Rust enum variants.",

        Rule::EnumMembers => "Separate multiline C, C++, TypeScript, and PowerShell enum members.",

        Rule::Services => "Separate Luau service groups from other module groups.",
        Rule::Requires => "Separate Luau require groups by module path prefix.",
        Rule::TypeGroups => "Separate imported, local, and exported Luau type groups.",
        Rule::ClassGroups => "Separate Luau class groups from other module groups.",

        Rule::PopulatedDeclarations => {
            "Separate Luau declarations populated by following field or index assignments."
        }
    }
}
