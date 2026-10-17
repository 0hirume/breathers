use super::{Configuration, Rule};
use crate::languages::Language;

macro_rules! fixture {
    ($directory:literal, $extension:literal, $name:literal) => {
        include_str!(concat!(
            "../../tests/fixtures/configuration/",
            $directory,
            "/",
            $name,
            ".",
            $extension
        ))
    };
}

const OVERRIDES: &[Override] = &[
    Override {
        language: Language::Rust,
        section: "rust",
        group: "returns",
        specific: "tail_expressions",
        source: fixture!("overrides/rust/tail", "rs", "before"),
        formatted: fixture!("overrides/rust/tail", "rs", "after"),
    },
    Override {
        language: Language::Rust,
        section: "rust",
        group: "returns",
        specific: "return_statements",
        source: fixture!("overrides/rust/return", "rs", "before"),
        formatted: fixture!("overrides/rust/return", "rs", "after"),
    },
    Override {
        language: Language::Rust,
        section: "rust",
        group: "arms",
        specific: "match_arms",
        source: fixture!("overrides/rust/match", "rs", "before"),
        formatted: fixture!("overrides/rust/match", "rs", "after"),
    },
    Override {
        language: Language::Rust,
        section: "rust",
        group: "variants",
        specific: "enum_variants",
        source: fixture!("overrides/rust/enum", "rs", "before"),
        formatted: fixture!("overrides/rust/enum", "rs", "after"),
    },
    Override {
        language: Language::Rust,
        section: "rust",
        group: "multiline",
        specific: "calls",
        source: fixture!("overrides/rust/call", "rs", "before"),
        formatted: fixture!("overrides/rust/call", "rs", "after"),
    },
    Override {
        language: Language::Lua,
        section: "lua",
        group: "blocks",
        specific: "for_loops",
        source: fixture!("overrides/lua/iteration", "lua", "before"),
        formatted: fixture!("overrides/lua/iteration", "lua", "after"),
    },
    Override {
        language: Language::Luau,
        section: "luau",
        group: "groups",
        specific: "requires",
        source: fixture!("overrides/luau/require", "luau", "before"),
        formatted: fixture!("overrides/luau/require", "luau", "after"),
    },
    Override {
        language: Language::Luau,
        section: "luau",
        group: "multiline",
        specific: "type_aliases",
        source: fixture!("overrides/luau/alias", "luau", "before"),
        formatted: fixture!("overrides/luau/alias", "luau", "after"),
    },
    Override {
        language: Language::C,
        section: "c",
        group: "blocks",
        specific: "while_loops",
        source: fixture!("overrides/c/iteration", "c", "before"),
        formatted: fixture!("overrides/c/iteration", "c", "after"),
    },
    Override {
        language: Language::CPlusPlus,
        section: "c++",
        group: "arms",
        specific: "switch_cases",
        source: fixture!("overrides/cplusplus/switch", "cpp", "before"),
        formatted: fixture!("overrides/cplusplus/switch", "cpp", "after"),
    },
    Override {
        language: Language::Python,
        section: "python",
        group: "blocks",
        specific: "decorated_functions",
        source: fixture!("overrides/python/decoration", "py", "before"),
        formatted: fixture!("overrides/python/decoration", "py", "after"),
    },
    Override {
        language: Language::Powershell,
        section: "powershell",
        group: "returns",
        specific: "return_statements",
        source: fixture!("overrides/powershell/return", "ps1", "before"),
        formatted: fixture!("overrides/powershell/return", "ps1", "after"),
    },
    Override {
        language: Language::Python,
        section: "python",
        group: "arms",
        specific: "match_cases",
        source: fixture!("overrides/python/match", "py", "before"),
        formatted: fixture!("overrides/python/match", "py", "after"),
    },
    Override {
        language: Language::Javascript,
        section: "javascript",
        group: "fields",
        specific: "object_properties",
        source: fixture!("overrides/javascript/object", "js", "before"),
        formatted: fixture!("overrides/javascript/object", "js", "after"),
    },
    Override {
        language: Language::Typescript,
        section: "typescript",
        group: "fields",
        specific: "interface_members",
        source: fixture!("overrides/typescript/interface", "ts", "before"),
        formatted: fixture!("overrides/typescript/interface", "ts", "after"),
    },
    Override {
        language: Language::Typescript,
        section: "typescript",
        group: "multiline",
        specific: "type_aliases",
        source: fixture!("overrides/typescript/alias", "ts", "before"),
        formatted: fixture!("overrides/typescript/alias", "ts", "after"),
    },
    Override {
        language: Language::Tsx,
        section: "typescript",
        group: "returns",
        specific: "return_statements",
        source: fixture!("overrides/typescript/rendering", "tsx", "before"),
        formatted: fixture!("overrides/typescript/rendering", "tsx", "after"),
    },
];

const RELATED: &[Related] = &[
    Related {
        language: Language::Python,
        section: "python",
        source: fixture!("related/python/comprehension", "py", "before"),
        baseline: fixture!("related/python/comprehension", "py", "default"),
        expected: fixture!("related/python/comprehension", "py", "after"),
        next: "result =",
    },
    Related {
        language: Language::Python,
        section: "python",
        source: fixture!("related/python/interpolation", "py", "before"),
        baseline: fixture!("related/python/interpolation", "py", "default"),
        expected: fixture!("related/python/interpolation", "py", "after"),
        next: "process(",
    },
    Related {
        language: Language::Python,
        section: "python",
        source: fixture!("related/python/assignment", "py", "before"),
        baseline: fixture!("related/python/assignment", "py", "default"),
        expected: fixture!("related/python/assignment", "py", "after"),
        next: "if second",
    },
    Related {
        language: Language::Rust,
        section: "rust",
        source: fixture!("related/rust/object", "rs", "before"),
        baseline: fixture!("related/rust/object", "rs", "default"),
        expected: fixture!("related/rust/object", "rs", "after"),
        next: "    let object",
    },
    Related {
        language: Language::C,
        section: "c",
        source: fixture!("related/c/assignment", "c", "before"),
        baseline: fixture!("related/c/assignment", "c", "default"),
        expected: fixture!("related/c/assignment", "c", "after"),
        next: "    value +=",
    },
    Related {
        language: Language::Luau,
        section: "luau",
        source: fixture!("related/luau/iteration", "luau", "before"),
        baseline: fixture!("related/luau/iteration", "luau", "default"),
        expected: fixture!("related/luau/iteration", "luau", "after"),
        next: "for key",
    },
    Related {
        language: Language::Lua,
        section: "lua",
        source: fixture!("related/lua/iteration", "lua", "before"),
        baseline: fixture!("related/lua/iteration", "lua", "default"),
        expected: fixture!("related/lua/iteration", "lua", "after"),
        next: "for key",
    },
    Related {
        language: Language::Luau,
        section: "luau",
        source: fixture!("related/luau/conditional", "luau", "before"),
        baseline: fixture!("related/luau/conditional", "luau", "default"),
        expected: fixture!("related/luau/conditional", "luau", "after"),
        next: "if ready",
    },
    Related {
        language: Language::Luau,
        section: "luau",
        source: fixture!("related/luau/destructuring", "luau", "before"),
        baseline: fixture!("related/luau/destructuring", "luau", "default"),
        expected: fixture!("related/luau/destructuring", "luau", "after"),
        next: "process(",
    },
    Related {
        language: Language::Luau,
        section: "luau",
        source: fixture!("related/luau/return", "luau", "before"),
        baseline: fixture!("related/luau/return", "luau", "default"),
        expected: fixture!("related/luau/return", "luau", "after"),
        next: "return value",
    },
    Related {
        language: Language::Luau,
        section: "luau",
        source: fixture!("related/luau/population", "luau", "before"),
        baseline: fixture!("related/luau/population", "luau", "default"),
        expected: fixture!("related/luau/population", "luau", "after"),
        next: "values.second",
    },
    Related {
        language: Language::Rust,
        section: "rust",
        source: fixture!("related/rust/iteration", "rs", "before"),
        baseline: fixture!("related/rust/iteration", "rs", "default"),
        expected: fixture!("related/rust/iteration", "rs", "after"),
        next: "    for value",
    },
    Related {
        language: Language::Rust,
        section: "rust",
        source: fixture!("related/rust/return", "rs", "before"),
        baseline: fixture!("related/rust/return", "rs", "default"),
        expected: fixture!("related/rust/return", "rs", "after"),
        next: "    return value",
    },
    Related {
        language: Language::Rust,
        section: "rust",
        source: fixture!("related/rust/destructuring", "rs", "before"),
        baseline: fixture!("related/rust/destructuring", "rs", "default"),
        expected: fixture!("related/rust/destructuring", "rs", "after"),
        next: "    second",
    },
    Related {
        language: Language::C,
        section: "c",
        source: fixture!("related/c/conditional", "c", "before"),
        baseline: fixture!("related/c/conditional", "c", "default"),
        expected: fixture!("related/c/conditional", "c", "after"),
        next: "    if (ready)",
    },
    Related {
        language: Language::CPlusPlus,
        section: "c++",
        source: fixture!("related/cplusplus/iteration", "cpp", "before"),
        baseline: fixture!("related/cplusplus/iteration", "cpp", "default"),
        expected: fixture!("related/cplusplus/iteration", "cpp", "after"),
        next: "    for (auto",
    },
    Related {
        language: Language::Python,
        section: "python",
        source: fixture!("related/python/iteration", "py", "before"),
        baseline: fixture!("related/python/iteration", "py", "default"),
        expected: fixture!("related/python/iteration", "py", "after"),
        next: "for value",
    },
    Related {
        language: Language::Python,
        section: "python",
        source: fixture!("related/python/return", "py", "before"),
        baseline: fixture!("related/python/return", "py", "default"),
        expected: fixture!("related/python/return", "py", "after"),
        next: "    return value",
    },
    Related {
        language: Language::Javascript,
        section: "javascript",
        source: fixture!("related/javascript/iteration", "js", "before"),
        baseline: fixture!("related/javascript/iteration", "js", "default"),
        expected: fixture!("related/javascript/iteration", "js", "after"),
        next: "for (const",
    },
    Related {
        language: Language::Typescript,
        section: "typescript",
        source: fixture!("related/typescript/call", "ts", "before"),
        baseline: fixture!("related/typescript/call", "ts", "default"),
        expected: fixture!("related/typescript/call", "ts", "after"),
        next: "process(",
    },
    Related {
        language: Language::Tsx,
        section: "typescript",
        source: fixture!("related/typescript/rendering", "tsx", "before"),
        baseline: fixture!("related/typescript/rendering", "tsx", "default"),
        expected: fixture!("related/typescript/rendering", "tsx", "after"),
        next: "    return",
    },
];

const UNRELATED: &[(Language, &str, &str, &str)] = &[
    (
        Language::Rust,
        "rust",
        fixture!("unrelated/rust/parentheses", "rs", "before"),
        fixture!("unrelated/rust/parentheses", "rs", "after"),
    ),
    (
        Language::Luau,
        "luau",
        fixture!("unrelated/luau/property", "luau", "before"),
        fixture!("unrelated/luau/property", "luau", "after"),
    ),
    (
        Language::Luau,
        "luau",
        fixture!("unrelated/luau/scope", "luau", "before"),
        fixture!("unrelated/luau/scope", "luau", "after"),
    ),
    (
        Language::Luau,
        "luau",
        fixture!("unrelated/luau/function", "luau", "before"),
        fixture!("unrelated/luau/function", "luau", "after"),
    ),
    (
        Language::Luau,
        "luau",
        fixture!("unrelated/luau/literal", "luau", "before"),
        fixture!("unrelated/luau/literal", "luau", "after"),
    ),
    (
        Language::Lua,
        "lua",
        fixture!("unrelated/lua/property", "lua", "before"),
        fixture!("unrelated/lua/property", "lua", "after"),
    ),
    (
        Language::Lua,
        "lua",
        fixture!("unrelated/lua/literal", "lua", "before"),
        fixture!("unrelated/lua/literal", "lua", "after"),
    ),
    (
        Language::Rust,
        "rust",
        fixture!("unrelated/rust/property", "rs", "before"),
        fixture!("unrelated/rust/property", "rs", "after"),
    ),
    (
        Language::Rust,
        "rust",
        fixture!("unrelated/rust/scope", "rs", "before"),
        fixture!("unrelated/rust/scope", "rs", "after"),
    ),
    (
        Language::Rust,
        "rust",
        fixture!("unrelated/rust/assignment", "rs", "before"),
        fixture!("unrelated/rust/assignment", "rs", "after"),
    ),
    (
        Language::C,
        "c",
        fixture!("unrelated/c/property", "c", "before"),
        fixture!("unrelated/c/property", "c", "after"),
    ),
    (
        Language::C,
        "c",
        fixture!("unrelated/c/shadowing", "c", "before"),
        fixture!("unrelated/c/shadowing", "c", "after"),
    ),
    (
        Language::CPlusPlus,
        "c++",
        fixture!("unrelated/cplusplus/scope", "cpp", "before"),
        fixture!("unrelated/cplusplus/scope", "cpp", "after"),
    ),
    (
        Language::Python,
        "python",
        fixture!("unrelated/python/property", "py", "before"),
        fixture!("unrelated/python/property", "py", "after"),
    ),
    (
        Language::Python,
        "python",
        fixture!("unrelated/python/scope", "py", "before"),
        fixture!("unrelated/python/scope", "py", "after"),
    ),
    (
        Language::Python,
        "python",
        fixture!("unrelated/python/literal", "py", "before"),
        fixture!("unrelated/python/literal", "py", "after"),
    ),
    (
        Language::Python,
        "python",
        fixture!("unrelated/python/comprehension", "py", "before"),
        fixture!("unrelated/python/comprehension", "py", "after"),
    ),
    (
        Language::Javascript,
        "javascript",
        fixture!("unrelated/javascript/property", "js", "before"),
        fixture!("unrelated/javascript/property", "js", "after"),
    ),
    (
        Language::Javascript,
        "javascript",
        fixture!("unrelated/javascript/assignment", "js", "before"),
        fixture!("unrelated/javascript/assignment", "js", "after"),
    ),
    (
        Language::Typescript,
        "typescript",
        fixture!("unrelated/typescript/conditional", "ts", "before"),
        fixture!("unrelated/typescript/conditional", "ts", "after"),
    ),
    (
        Language::Tsx,
        "typescript",
        fixture!("unrelated/typescript/rendering", "tsx", "before"),
        fixture!("unrelated/typescript/rendering", "tsx", "after"),
    ),
];

const DISABLING: &[(Language, &str, &str, &str, &str)] = &[
    (
        Language::Rust,
        "rust",
        "blocks",
        fixture!("disabling/rust/conditional", "rs", "before"),
        fixture!("disabling/rust/conditional", "rs", "after"),
    ),
    (
        Language::Rust,
        "rust",
        "multiline",
        fixture!("disabling/rust/call", "rs", "before"),
        fixture!("disabling/rust/call", "rs", "after"),
    ),
    (
        Language::Rust,
        "rust",
        "returns",
        fixture!("disabling/rust/tail", "rs", "before"),
        fixture!("disabling/rust/tail", "rs", "after"),
    ),
    (
        Language::Rust,
        "rust",
        "fields",
        fixture!("disabling/rust/field", "rs", "before"),
        fixture!("disabling/rust/field", "rs", "after"),
    ),
    (
        Language::Rust,
        "rust",
        "variants",
        fixture!("overrides/rust/enum", "rs", "before"),
        fixture!("overrides/rust/enum", "rs", "after"),
    ),
    (
        Language::Rust,
        "rust",
        "arms",
        fixture!("overrides/rust/match", "rs", "before"),
        fixture!("overrides/rust/match", "rs", "after"),
    ),
    (
        Language::Luau,
        "luau",
        "groups",
        fixture!("disabling/luau/service", "luau", "before"),
        fixture!("disabling/luau/service", "luau", "after"),
    ),
    (
        Language::Lua,
        "lua",
        "returns",
        fixture!("disabling/lua/return", "lua", "before"),
        fixture!("disabling/lua/return", "lua", "after"),
    ),
    (
        Language::Luau,
        "luau",
        "returns",
        fixture!("disabling/luau/return", "luau", "before"),
        fixture!("disabling/luau/return", "luau", "after"),
    ),
    (
        Language::C,
        "c",
        "returns",
        fixture!("disabling/c/return", "c", "before"),
        fixture!("disabling/c/return", "c", "after"),
    ),
    (
        Language::CPlusPlus,
        "c++",
        "returns",
        fixture!("disabling/cplusplus/return", "cpp", "before"),
        fixture!("disabling/cplusplus/return", "cpp", "after"),
    ),
    (
        Language::Python,
        "python",
        "returns",
        fixture!("disabling/python/return", "py", "before"),
        fixture!("disabling/python/return", "py", "after"),
    ),
    (
        Language::Javascript,
        "javascript",
        "returns",
        fixture!("disabling/javascript/return", "js", "before"),
        fixture!("disabling/javascript/return", "js", "after"),
    ),
    (
        Language::Typescript,
        "typescript",
        "returns",
        fixture!("disabling/typescript/return", "ts", "before"),
        fixture!("disabling/typescript/return", "ts", "after"),
    ),
    (
        Language::Tsx,
        "typescript",
        "returns",
        fixture!("overrides/typescript/rendering", "tsx", "before"),
        fixture!("overrides/typescript/rendering", "tsx", "after"),
    ),
];

struct Override {
    language: Language,
    section: &'static str,
    group: &'static str,
    specific: &'static str,
    source: &'static str,
    formatted: &'static str,
}

struct Related {
    language: Language,
    section: &'static str,
    source: &'static str,
    baseline: &'static str,
    expected: &'static str,
    next: &'static str,
}

#[test]
fn keeps_adjacent_dependencies_together_without_removing_blank_lines() {
    for &Related {
        language,
        section,
        source,
        baseline,
        expected,
        next,
    } in RELATED
    {
        let configuration: Configuration =
            toml::from_str(&format!("[\"{section}\"]\nrelated = true\n")).unwrap();

        assert_eq!(
            language.format(source, &Configuration::default()).unwrap(),
            baseline,
            "{section}: {source}"
        );

        let boundary = format!("\n\n{next}");
        assert!(baseline.contains(&boundary), "{section}: {source}");

        assert_eq!(
            language.format(source, &configuration).unwrap(),
            expected,
            "{section}: {source}"
        );

        assert_eq!(language.format(expected, &configuration).unwrap(), expected);

        assert_eq!(language.format(baseline, &configuration).unwrap(), baseline);

        assert_eq!(
            language
                .format(&source.replace('\n', "\r\n"), &configuration)
                .unwrap(),
            expected.replace('\n', "\r\n")
        );
    }
}

#[test]
fn related_keeps_dependency_chains_together() {
    let source = r"local first = build(
    input
)
local second = first
return second
";
    let configuration = toml::from_str("[luau]\nrelated = true\n").unwrap();

    assert_eq!(
        Language::Luau.format(source, &configuration).unwrap(),
        source
    );

    assert!(!Configuration::default().luau.enabled(Rule::Related));

    assert_ne!(
        Language::Luau
            .format(source, &Configuration::default())
            .unwrap(),
        source
    );
}

#[test]
fn related_ignores_properties_literals_writes_and_nested_scopes() {
    for &(language, section, source, expected) in UNRELATED {
        let configuration: Configuration =
            toml::from_str(&format!("[\"{section}\"]\nrelated = true\n")).unwrap();

        assert_eq!(
            language.format(source, &Configuration::default()).unwrap(),
            expected,
            "{section}: {source}"
        );

        assert_ne!(expected, source, "{section}: {source}");

        assert_eq!(
            language.format(source, &configuration).unwrap(),
            expected,
            "{section}: {source}"
        );
    }
}

#[test]
fn granular_overrides_take_precedence_in_both_directions() {
    for &Override {
        language,
        section,
        group,
        specific,
        source,
        formatted,
    } in OVERRIDES
    {
        assert_eq!(
            language.format(source, &Configuration::default()).unwrap(),
            formatted,
            "{section}.{specific}"
        );

        assert_ne!(formatted, source, "{section}.{specific}");

        for (group_value, override_value, expected) in [
            (false, None, source),
            (false, Some(true), formatted),
            (true, Some(false), source),
        ] {
            let override_text =
                override_value.map_or_default(|value| format!("{specific} = {value}\n"));

            let text = format!("[\"{section}\"]\n{group} = {group_value}\n{override_text}");
            let configuration: Configuration = toml::from_str(&text).unwrap();

            assert_eq!(
                language.format(source, &configuration).unwrap(),
                expected,
                "{text}"
            );
        }
    }
}

#[test]
fn disables_each_rule_without_removing_existing_blank_lines() {
    for &(language, section, rule, source, formatted) in DISABLING {
        assert_eq!(
            language.format(source, &Configuration::default()).unwrap(),
            formatted,
            "{section}.{rule}"
        );

        assert_ne!(formatted, source, "{section}.{rule}");

        let configuration: Configuration =
            toml::from_str(&format!("[\"{section}\"]\n{rule} = false\n")).unwrap();

        assert_eq!(
            language.format(source, &configuration).unwrap(),
            source,
            "{section}.{rule}"
        );

        assert_eq!(
            language.format(formatted, &configuration).unwrap(),
            formatted
        );
    }
}

#[test]
fn controls_fields_arms_and_variants_in_new_languages() {
    for (language, section, rule, source) in [
        (
            Language::Python,
            "python",
            "fields",
            fixture!("disabling/python/dictionary", "py", "before"),
        ),
        (
            Language::Python,
            "python",
            "arms",
            fixture!("overrides/python/match", "py", "before"),
        ),
        (
            Language::Javascript,
            "javascript",
            "fields",
            fixture!("overrides/javascript/object", "js", "before"),
        ),
        (
            Language::Javascript,
            "javascript",
            "arms",
            fixture!("disabling/javascript/switch", "js", "before"),
        ),
        (
            Language::Typescript,
            "typescript",
            "fields",
            fixture!("overrides/typescript/interface", "ts", "before"),
        ),
        (
            Language::Typescript,
            "typescript",
            "variants",
            fixture!("disabling/typescript/enum", "ts", "before"),
        ),
    ] {
        if language == Language::Python && rule == "fields" {
            assert_eq!(
                language.format(source, &Configuration::default()).unwrap(),
                source
            );
        } else {
            assert_ne!(
                language.format(source, &Configuration::default()).unwrap(),
                source
            );
        }

        let configuration: Configuration =
            toml::from_str(&format!("[{section}]\n{rule} = true")).unwrap();

        assert_ne!(language.format(source, &configuration).unwrap(), source);

        if language == Language::Python && rule == "fields" {
            let configuration: Configuration =
                toml::from_str("[python]\ndictionary_entries = true").unwrap();

            assert_ne!(language.format(source, &configuration).unwrap(), source);
        }

        let configuration: Configuration =
            toml::from_str(&format!("[{section}]\n{rule} = false")).unwrap();

        assert_eq!(language.format(source, &configuration).unwrap(), source);
    }
}
