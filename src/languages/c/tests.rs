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
    let source = r"struct Value { Value() = default; };
int example(Value parent = {}) {
    int first = 1; // trailing
    // attached
    if (first) {
        ++first;
    }
    return first;
}
";

    let expected = r"struct Value { Value() = default; };

int example(Value parent = {}) {
    int first = 1; // trailing

    // attached
    if (first) {
        ++first;
    }

    return first;
}
";

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
    let source = r"int resolve(
    const int *from, int *expression, const int *limits
) {
    if (!from) {
        return 0;
    }
    return expression ? *from : 0;
}
";
    let expected = r"int resolve(
    const int *from, int *expression, const int *limits
) {
    if (!from) {
        return 0;
    }

    return expression ? *from : 0;
}
";
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
    let source = r"struct Sources {
    int resolve(
        int *from, int *expression, int &) {
        if (!from) {
            return 0;
        }
        return expression ? *from : 0;
    }
};
";

    let expected = r"struct Sources {
    int resolve(
        int *from, int *expression, int &) {
        if (!from) {
            return 0;
        }

        return expression ? *from : 0;
    }
};
";

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
    let source = r"struct Sources {
    Sources(int count) {
        for (int index = 0; index < count; ++index) {
            (void)index;
        }
    }
};
";

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
            r"int
example(const int *value) {
    if (!value) {
        return 0;
    }

    return *value;
}
",
            Dialect::C,
        ),
        (
            r"int
example(const int *value) {
    if (!value) {
        return 0;
    }

    return *value;
}
",
            Dialect::CPlusPlus,
        ),
        (
            r"struct Example {
    int resolve(const int *value);
};

int
Example::resolve(const int *value) {
    if (!value) {
        return 0;
    }

    return *value;
}
",
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
    let source = r"void report_error(const char *message) {
    if (!message) {
        return;
    }
}
";

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
            r"void example() {
    int value = 1;
    value = (
        2
    );
}
",
            "    value =",
        ),
        (
            r"void example() {
    int index = 1;
    for (int index = 0; index < 10; ++index) {
    }
}
",
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
    let source = r"#define IDENTITY(value) value
int example() {
    auto callback = IDENTITY([] {
        int first = 1;
        return first;
    });
    return callback();
}
";

    assert_eq!(
        super::breathe(source, None, &Rules::default(), Dialect::CPlusPlus).unwrap(),
        r"#define IDENTITY(value) value
int example() {
    auto callback = IDENTITY([] {
        int first = 1;
        return first;
    });

    return callback();
}
"
    );
}

#[test]
fn preserves_macros_and_literal_contents() {
    let source = r#"#define BODY do { int first = 1; if (first) { ++first; } } while (0)
int example() {
    BODY;
    const char *text = R"(one
two)";
    return text[0];
}
"#;
    let expected = r#"#define BODY do { int first = 1; if (first) { ++first; } } while (0)
int example() {
    BODY;
    const char *text = R"(one
two)";

    return text[0];
}
"#;

    assert_eq!(
        super::breathe(source, None, &Rules::default(), Dialect::CPlusPlus).unwrap(),
        expected
    );
}
