use crate::configuration::{Configuration, Rules};

use super::Dialect;

#[test]
fn preserves_inactive_code_with_parser_fallback() {
    let source = include_str!("../../../tests/fixtures/c/preprocessing/input.cpp");
    let expected = include_str!("../../../tests/fixtures/c/preprocessing/output.cpp");
    let rules = Rules::default();

    assert_eq!(
        super::breathe(source, None, &rules, Dialect::CPlusPlus).unwrap(),
        expected
    );

    assert_eq!(
        super::breathe(expected, None, &rules, Dialect::CPlusPlus).unwrap(),
        expected
    );

    assert_eq!(
        super::breathe(
            &source.replace('\n', "\r\n"),
            None,
            &rules,
            Dialect::CPlusPlus
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
        super::breathe(source, None, &rules, Dialect::CPlusPlus).unwrap(),
        expected
    );

    assert_eq!(
        super::breathe(expected, None, &rules, Dialect::CPlusPlus).unwrap(),
        expected
    );

    assert_eq!(
        super::breathe(
            &source.replace('\n', "\r\n"),
            None,
            &rules,
            Dialect::CPlusPlus
        )
        .unwrap(),
        expected.replace('\n', "\r\n")
    );

    super::breathe("int broken( {", None, &rules, Dialect::CPlusPlus).unwrap_err();
}

#[test]
fn keeps_first_control_flow_statement_adjacent() {
    let source = include_str!("../../../tests/fixtures/c/resolution/input.c");
    let expected = include_str!("../../../tests/fixtures/c/resolution/output.c");
    let rules = Rules::default();

    for language in [Dialect::C, Dialect::CPlusPlus] {
        assert_eq!(
            super::breathe(source, None, &rules, language).unwrap(),
            expected
        );

        assert_eq!(
            super::breathe(expected, None, &rules, language).unwrap(),
            expected
        );

        assert_eq!(
            super::breathe(&source.replace('\n', "\r\n"), None, &rules, language).unwrap(),
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
        super::breathe(source, None, &rules, Dialect::CPlusPlus).unwrap(),
        expected
    );

    assert_eq!(
        super::breathe(expected, None, &rules, Dialect::CPlusPlus).unwrap(),
        expected
    );
}

#[test]
fn leaves_first_constructor_loop_adjacent() {
    let source = include_str!("../../../tests/fixtures/c/constructor/input.cpp");

    let rules = Rules::default();

    assert_eq!(
        super::breathe(source, None, &rules, Dialect::CPlusPlus).unwrap(),
        source
    );

    let windows = source.replace('\n', "\r\n");

    assert_eq!(
        super::breathe(&windows, None, &rules, Dialect::CPlusPlus).unwrap(),
        windows
    );
}

#[test]
fn leaves_first_statement_adjacent_after_wrapped_return_type() {
    let rules = Rules::default();

    for (source, language) in [
        (
            include_str!("../../../tests/fixtures/c/returning/input.c"),
            Dialect::C,
        ),
        (
            include_str!("../../../tests/fixtures/c/returning/input.c"),
            Dialect::CPlusPlus,
        ),
        (
            include_str!("../../../tests/fixtures/c/member/input.cpp"),
            Dialect::CPlusPlus,
        ),
    ] {
        assert_eq!(
            super::breathe(source, None, &rules, language).unwrap(),
            source
        );

        let windows = source.replace('\n', "\r\n");

        assert_eq!(
            super::breathe(&windows, None, &rules, language).unwrap(),
            windows
        );
    }
}

#[test]
fn leaves_first_statement_adjacent_for_single_line_header() {
    let source = include_str!("../../../tests/fixtures/c/header/input.cpp");

    let rules = Rules::default();

    assert_eq!(
        super::breathe(source, None, &rules, Dialect::CPlusPlus).unwrap(),
        source
    );

    let windows = source.replace('\n', "\r\n");

    assert_eq!(
        super::breathe(&windows, None, &rules, Dialect::CPlusPlus).unwrap(),
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
        super::breathe(source, None, rules, Dialect::CPlusPlus).unwrap(),
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
        let formatted = super::breathe(source, None, rules, Dialect::CPlusPlus).unwrap();
        assert!(formatted.contains(&format!("\n\n{next}")), "{formatted}");

        assert_eq!(
            super::breathe(&formatted, None, rules, Dialect::CPlusPlus).unwrap(),
            formatted
        );
    }
}

#[test]
fn preserves_macro_arguments() {
    let source = include_str!("../../../tests/fixtures/c/arguments/input.cpp");

    assert_eq!(
        super::breathe(source, None, &Rules::default(), Dialect::CPlusPlus).unwrap(),
        include_str!("../../../tests/fixtures/c/arguments/output.cpp")
    );
}

#[test]
fn preserves_macros_and_literal_contents() {
    let source = include_str!("../../../tests/fixtures/c/literal/input.cpp");
    let expected = include_str!("../../../tests/fixtures/c/literal/output.cpp");

    assert_eq!(
        super::breathe(source, None, &Rules::default(), Dialect::CPlusPlus).unwrap(),
        expected
    );
}
