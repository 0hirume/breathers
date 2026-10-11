use crate::configuration::{Configuration, Rules};

#[test]
fn preserves_inactive_code() {
    let source = include_str!("../../../tests/fixtures/c/preprocessing/input.cpp");
    let expected = include_str!("../../../tests/fixtures/c/preprocessing/output.cpp");
    let rules = Rules::default();

    assert_eq!(
        super::breathe(source, &tree_sitter_cpp::LANGUAGE.into(), &rules).unwrap(),
        expected
    );

    assert_eq!(
        super::breathe(expected, &tree_sitter_cpp::LANGUAGE.into(), &rules).unwrap(),
        expected
    );

    assert_eq!(
        super::breathe(
            &source.replace('\n', "\r\n"),
            &tree_sitter_cpp::LANGUAGE.into(),
            &rules,
        )
        .unwrap(),
        expected.replace('\n', "\r\n")
    );
}

#[test]
fn formats_braced_defaults_and_preserves_source() {
    let source = include_str!("../../../tests/fixtures/c/defaults/input.cpp");

    let expected = include_str!("../../../tests/fixtures/c/defaults/output.cpp");

    let rules = Rules::default();

    assert_eq!(
        super::breathe(source, &tree_sitter_cpp::LANGUAGE.into(), &rules).unwrap(),
        expected
    );

    assert_eq!(
        super::breathe(expected, &tree_sitter_cpp::LANGUAGE.into(), &rules).unwrap(),
        expected
    );

    assert_eq!(
        super::breathe(
            &source.replace('\n', "\r\n"),
            &tree_sitter_cpp::LANGUAGE.into(),
            &rules,
        )
        .unwrap(),
        expected.replace('\n', "\r\n")
    );

    super::breathe("int broken( {", &tree_sitter_cpp::LANGUAGE.into(), &rules).unwrap_err();
}

#[test]
fn keeps_first_control_flow_statement_adjacent() {
    let source = include_str!("../../../tests/fixtures/c/resolution/input.c");
    let expected = include_str!("../../../tests/fixtures/c/resolution/output.c");
    let rules = Rules::default();

    for language in [
        tree_sitter_c::LANGUAGE.into(),
        tree_sitter_cpp::LANGUAGE.into(),
    ] {
        assert_eq!(super::breathe(source, &language, &rules).unwrap(), expected);

        assert_eq!(
            super::breathe(expected, &language, &rules).unwrap(),
            expected
        );

        assert_eq!(
            super::breathe(&source.replace('\n', "\r\n"), &language, &rules).unwrap(),
            expected.replace('\n', "\r\n")
        );
    }
}

#[test]
fn keeps_first_control_flow_statement_adjacent_in_method() {
    let source = include_str!("../../../tests/fixtures/c/method/input.cpp");

    let expected = include_str!("../../../tests/fixtures/c/method/output.cpp");

    let rules = Rules::default();

    assert_eq!(
        super::breathe(source, &tree_sitter_cpp::LANGUAGE.into(), &rules).unwrap(),
        expected
    );

    assert_eq!(
        super::breathe(expected, &tree_sitter_cpp::LANGUAGE.into(), &rules).unwrap(),
        expected
    );
}

#[test]
fn leaves_first_constructor_loop_adjacent() {
    let source = include_str!("../../../tests/fixtures/c/constructor/input.cpp");

    let rules = Rules::default();

    assert_eq!(
        super::breathe(source, &tree_sitter_cpp::LANGUAGE.into(), &rules).unwrap(),
        source
    );

    let windows = source.replace('\n', "\r\n");

    assert_eq!(
        super::breathe(&windows, &tree_sitter_cpp::LANGUAGE.into(), &rules).unwrap(),
        windows
    );
}

#[test]
fn leaves_first_statement_adjacent_after_wrapped_return_type() {
    let rules = Rules::default();

    for (source, language) in [
        (
            include_str!("../../../tests/fixtures/c/returning/input.c"),
            tree_sitter_c::LANGUAGE.into(),
        ),
        (
            include_str!("../../../tests/fixtures/c/returning/input.c"),
            tree_sitter_cpp::LANGUAGE.into(),
        ),
        (
            include_str!("../../../tests/fixtures/c/member/input.cpp"),
            tree_sitter_cpp::LANGUAGE.into(),
        ),
    ] {
        assert_eq!(super::breathe(source, &language, &rules).unwrap(), source);

        let windows = source.replace('\n', "\r\n");

        assert_eq!(
            super::breathe(&windows, &language, &rules).unwrap(),
            windows
        );
    }
}

#[test]
fn leaves_first_statement_adjacent_for_single_line_header() {
    let source = include_str!("../../../tests/fixtures/c/header/input.cpp");

    let rules = Rules::default();

    assert_eq!(
        super::breathe(source, &tree_sitter_cpp::LANGUAGE.into(), &rules).unwrap(),
        source
    );

    let windows = source.replace('\n', "\r\n");

    assert_eq!(
        super::breathe(&windows, &tree_sitter_cpp::LANGUAGE.into(), &rules).unwrap(),
        windows
    );
}

#[test]
fn respects_related_reads_and_shadowed_bindings() {
    let configuration: Configuration = toml::from_str("[\"c++\"]\nrelated = true\n").unwrap();

    let rules = &configuration.cplusplus;
    let source = r"int example() {
    int value = 1;
    return value;
}
";

    assert_eq!(
        super::breathe(source, &tree_sitter_cpp::LANGUAGE.into(), rules).unwrap(),
        source
    );

    for (source, next) in [
        (
            include_str!("../../../tests/fixtures/c/assignment/input.cpp"),
            "    value =",
        ),
        (
            include_str!("../../../tests/fixtures/c/shadowing/input.cpp"),
            "    for",
        ),
    ] {
        let formatted = super::breathe(source, &tree_sitter_cpp::LANGUAGE.into(), rules).unwrap();
        assert!(formatted.contains(&format!("\n\n{next}")), "{formatted}");

        assert_eq!(
            super::breathe(&formatted, &tree_sitter_cpp::LANGUAGE.into(), rules).unwrap(),
            formatted
        );
    }
}

#[test]
fn preserves_macro_arguments() {
    let source = include_str!("../../../tests/fixtures/c/arguments/input.cpp");

    assert_eq!(
        super::breathe(source, &tree_sitter_cpp::LANGUAGE.into(), &Rules::default()).unwrap(),
        include_str!("../../../tests/fixtures/c/arguments/output.cpp")
    );
}

#[test]
fn preserves_macros_and_literal_contents() {
    let source = include_str!("../../../tests/fixtures/c/literal/input.cpp");
    let expected = include_str!("../../../tests/fixtures/c/literal/output.cpp");

    assert_eq!(
        super::breathe(source, &tree_sitter_cpp::LANGUAGE.into(), &Rules::default()).unwrap(),
        expected
    );
}

fn breathe(source: &str) -> Result<String, String> {
    super::breathe(source, &tree_sitter_cpp::LANGUAGE.into(), &Rules::default())
}

#[test]
fn spaces_cplusplus_statements() {
    let source = include_str!("../../../tests/fixtures/cplusplus/statements/input.cpp");

    let expected = include_str!("../../../tests/fixtures/cplusplus/statements/output.cpp");

    assert_eq!(breathe(source).unwrap(), expected);
    assert_eq!(breathe(expected).unwrap(), expected);

    assert_eq!(
        breathe(&source.replace('\n', "\r\n")).unwrap(),
        expected.replace('\n', "\r\n")
    );

    breathe("int broken( {").unwrap_err();
}

#[test]
fn preserves_directives_literals_and_macro_arguments() {
    for (source, expected) in [
        (
            r"#define EXAMPLE(x) do { first(); second(); } while (0)
",
            r"#define EXAMPLE(x) do { first(); second(); } while (0)
",
        ),
        (
            r"#define EXAMPLE(x) \
    first(); \
    second();
",
            r"#define EXAMPLE(x) \
    first(); \
    second();
",
        ),
        (
            include_str!("../../../tests/fixtures/cplusplus/arguments/input.cpp"),
            include_str!("../../../tests/fixtures/cplusplus/arguments/output.cpp"),
        ),
        (
            include_str!("../../../tests/fixtures/cplusplus/literal/input.cpp"),
            include_str!("../../../tests/fixtures/cplusplus/literal/input.cpp"),
        ),
        (
            r"void example() { first(); if (ready()) { second(); } third(); }
",
            r"void example() { first(); if (ready()) { second(); } third(); }
",
        ),
    ] {
        assert_eq!(breathe(source).unwrap(), expected);
    }
}

#[test]
fn spaces_fields_cases_and_guarded_code() {
    let source = include_str!("../../../tests/fixtures/cplusplus/guard/input.cpp");

    assert_eq!(
        breathe(source).unwrap(),
        include_str!("../../../tests/fixtures/cplusplus/guard/output.cpp")
    );

    let source = include_str!("../../../tests/fixtures/cplusplus/fields/input.cpp");

    let expected = include_str!("../../../tests/fixtures/cplusplus/fields/output.cpp");

    assert_eq!(breathe(source).unwrap(), expected);

    let source =
        include_str!("../../../tests/fixtures/configuration/overrides/cplusplus/switch/before.cpp");

    let expected =
        include_str!("../../../tests/fixtures/configuration/overrides/cplusplus/switch/after.cpp");

    assert_eq!(breathe(source).unwrap(), expected);
}

#[test]
fn protects_inactive_code_without_pruning_active_code() {
    let source = include_str!("../../../tests/fixtures/cplusplus/inactive/input.cpp");
    let expected = include_str!("../../../tests/fixtures/cplusplus/inactive/output.cpp");

    assert_eq!(breathe(source).unwrap(), expected);

    assert_eq!(breathe(expected).unwrap(), expected);
}

#[test]
fn preserves_macro_invocations_and_formats_surrounding_code() {
    let source = include_str!("../../../tests/fixtures/c/macros/input.c");
    let expected = include_str!("../../../tests/fixtures/c/macros/output.c");
    let rules = Rules::default();

    for language in [
        tree_sitter_c::LANGUAGE.into(),
        tree_sitter_cpp::LANGUAGE.into(),
    ] {
        assert_eq!(super::breathe(source, &language, &rules).unwrap(), expected);

        assert_eq!(
            super::breathe(expected, &language, &rules).unwrap(),
            expected
        );

        let invalid = source.replace("return value;", "return value +;");
        super::breathe(&invalid, &language, &rules).unwrap_err();
    }
}
