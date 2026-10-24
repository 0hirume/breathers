use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(
    Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Deserialize, JsonSchema, Serialize,
)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Rule {
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
    LoopStatements,
    Modules,
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
    Pipelines,
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
    RecordFields,
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
    pub(super) fn parent(self) -> Option<Self> {
        match self {
            Self::Conditionals
            | Self::ForLoops
            | Self::WhileLoops
            | Self::RepeatLoops
            | Self::LoopExpressions
            | Self::LoopStatements
            | Self::Modules
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
            | Self::Pipelines
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
            | Self::DictionaryEntries
            | Self::RecordFields => Some(Self::Fields),

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

#[derive(Default, Deserialize, JsonSchema, Serialize)]
#[serde(transparent)]
pub(crate) struct Rules(pub(super) BTreeMap<Rule, bool>);

impl Rules {
    pub(crate) fn enabled(&self, rule: Rule) -> bool {
        self.enabled_or(rule, rule != Rule::Related)
    }

    pub(crate) fn enabled_or(&self, rule: Rule, default: bool) -> bool {
        self.0.get(&rule).copied().unwrap_or_else(|| {
            rule.parent()
                .map_or(default, |parent| self.enabled_or(parent, default))
        })
    }
}
