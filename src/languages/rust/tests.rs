use crate::configuration::{Configuration, Rules};

#[test]
fn spaces_multiline_items() {
    let source = include_str!("../../../tests/fixtures/rust/items/input.rs");
    let expected = include_str!("../../../tests/fixtures/rust/items/output.rs");
    let rules = Rules::default();

    let configuration: Configuration = toml::from_str(
        r"[rust]
multiline = false
",
    )
    .unwrap();

    for (opening, closing) in [
        ("", ""),
        (
            r"mod example {
",
            r"}
",
        ),
    ] {
        let source = format!("{opening}{source}{closing}");
        let expected = format!("{opening}{expected}{closing}");
        assert_eq!(super::breathe(&source, &rules).unwrap(), expected);
        assert_eq!(super::breathe(&expected, &rules).unwrap(), expected);

        assert_eq!(
            super::breathe(&source, &configuration.rust).unwrap(),
            source
        );

        assert_eq!(
            super::breathe(&expected, &configuration.rust).unwrap(),
            expected
        );

        assert_eq!(
            super::breathe(&source.replace('\n', "\r\n"), &rules).unwrap(),
            expected.replace('\n', "\r\n")
        );
    }
}

#[test]
fn spaces_multiline_extern_items() {
    let expected = r#"extern "Rust" {
    type Context<'host>;
    fn source(self: &Context<'_>, module: &str) -> Result<Text>;

    fn child(
        self: &Context<'_>,
        module: &str,
        name: &str,
        destination: &mut String,
    ) -> Result<Status>;

    fn configuration(self: &Context<'_>, module: &str) -> Result<Configuration>;
}
"#;

    let source = expected.replace("\n\n", "\n");
    assert_eq!(breathe(&source).unwrap(), expected);
    assert_eq!(breathe(expected).unwrap(), expected);

    assert_eq!(
        breathe(&source.replace('\n', "\r\n")).unwrap(),
        expected.replace('\n', "\r\n")
    );

    let rules: Rules = toml::from_str("multiline = false").unwrap();
    assert_eq!(super::breathe(&source, &rules).unwrap(), source);
}

#[test]
fn spaces_function_items_in_files_modules_and_implementations() {
    let functions = include_str!("../../../tests/fixtures/rust/functions/input.rs");
    let expected = include_str!("../../../tests/fixtures/rust/functions/output.rs");

    let configuration: Configuration = toml::from_str(
        r"[rust]
functions = false
",
    )
    .unwrap();

    let rules = Rules::default();

    for (opening, closing) in [
        ("", ""),
        (
            r"mod example {
",
            r"}
",
        ),
        (
            r"impl Example {
",
            r"}
",
        ),
    ] {
        let source = format!("{opening}{functions}{closing}");
        let expected = format!("{opening}{expected}{closing}");
        assert_eq!(super::breathe(&source, &rules).unwrap(), expected);
        assert_eq!(super::breathe(&expected, &rules).unwrap(), expected);

        assert_eq!(
            super::breathe(&source, &configuration.rust).unwrap(),
            source
        );

        assert_eq!(
            super::breathe(&expected, &configuration.rust).unwrap(),
            expected
        );

        assert_eq!(
            super::breathe(&source.replace('\n', "\r\n"), &rules).unwrap(),
            expected.replace('\n', "\r\n")
        );
    }
}

#[test]
fn spaces_statements_without_changing_tokens() {
    let source = include_str!("../../../tests/fixtures/rust/statements/input.rs");
    let expected = include_str!("../../../tests/fixtures/rust/statements/output.rs");
    assert_eq!(breathe(source).unwrap(), expected);
    assert_eq!(breathe(expected).unwrap(), expected);

    assert_eq!(
        breathe(&source.replace('\n', "\r\n")).unwrap(),
        expected.replace('\n', "\r\n")
    );
}

#[test]
fn preserves_literals_macros_and_compact_code() {
    for source in [
        r"fn example() { one(); if ready() { two(); } three(); }
",
        include_str!("../../../tests/fixtures/rust/literal/input.rs"),
        r"macro_rules! example { () => { first(); if ready() { second(); } third(); }; }
",
        include_str!("../../../tests/fixtures/rust/attachment/input.rs"),
    ] {
        assert_eq!(breathe(source).unwrap(), source);
    }

    breathe("fn broken( {").unwrap_err();
}

#[test]
fn spaces_multiline_struct_fields() {
    let source = include_str!("../../../tests/fixtures/rust/structure/input.rs");
    let expected = include_str!("../../../tests/fixtures/rust/structure/output.rs");
    assert_eq!(breathe(source).unwrap(), expected);
    assert_eq!(breathe(expected).unwrap(), expected);

    assert_eq!(
        breathe(&source.replace('\n', "\r\n")).unwrap(),
        expected.replace('\n', "\r\n")
    );

    let source = include_str!("../../../tests/fixtures/rust/tuple/input.rs");
    let expected = include_str!("../../../tests/fixtures/rust/tuple/output.rs");
    assert_eq!(breathe(source).unwrap(), expected);

    for source in [
        r"struct Simple {
    first: u32,
    second: bool,
}
",
        include_str!("../../../tests/fixtures/rust/field/input.rs"),
    ] {
        assert_eq!(breathe(source).unwrap(), source);
    }
}

#[test]
fn spaces_returns_and_final_expressions() {
    for expression in [
        "return result;",
        "result",
        "Ok(result)",
        "return;",
        "return result",
    ] {
        let source = format!(
            include_str!("../../../tests/fixtures/rust/returns/input.rs"),
            expression = expression
        );

        let expected = format!(
            include_str!("../../../tests/fixtures/rust/returns/output.rs"),
            expression = expression
        );

        assert_eq!(breathe(&source).unwrap(), expected);
        assert_eq!(breathe(&expected).unwrap(), expected);
    }

    for source in [
        r"fn example() {
    return result;
}
",
        r"fn example() {
    result
}
",
        r"fn example() { first(); result }
",
        include_str!("../../../tests/fixtures/rust/closure/input.rs"),
    ] {
        assert_eq!(breathe(source).unwrap(), source);
    }
}

#[test]
fn spaces_multiline_enum_variants() {
    let source = include_str!("../../../tests/fixtures/rust/enumeration/input.rs");
    let expected = include_str!("../../../tests/fixtures/rust/enumeration/output.rs");
    assert_eq!(breathe(source).unwrap(), expected);
    assert_eq!(breathe(expected).unwrap(), expected);

    assert_eq!(
        breathe(&source.replace('\n', "\r\n")).unwrap(),
        expected.replace('\n', "\r\n")
    );

    for source in [
        r"enum Example { First, Second(u32), Record { value: u32 } }
",
        include_str!("../../../tests/fixtures/rust/compact/input.rs"),
        include_str!("../../../tests/fixtures/rust/variant/input.rs"),
    ] {
        assert_eq!(breathe(source).unwrap(), source);
    }
}

#[test]
fn spaces_multiline_match_arms() {
    let source = include_str!("../../../tests/fixtures/rust/matching/input.rs");
    let expected = include_str!("../../../tests/fixtures/rust/matching/output.rs");
    assert_eq!(breathe(source).unwrap(), expected);
    assert_eq!(breathe(expected).unwrap(), expected);

    assert_eq!(
        breathe(&source.replace('\n', "\r\n")).unwrap(),
        expected.replace('\n', "\r\n")
    );

    for source in [
        include_str!("../../../tests/fixtures/rust/arms/input.rs"),
        include_str!("../../../tests/fixtures/rust/text/input.rs"),
    ] {
        assert_eq!(breathe(source).unwrap(), source);
    }
}

#[test]
fn spaces_control_flow_and_tail_expressions() {
    for statement in [
        "if ready() {} else {}",
        "match value { _ => () }",
        "for value in values {}",
        "while ready() {}",
        "loop { break; }",
        "let value = if ready() { 1 } else { 2 };",
    ] {
        let source = format!(
            include_str!("../../../tests/fixtures/rust/flow/input.rs"),
            statement = statement
        );

        let expected = format!(
            include_str!("../../../tests/fixtures/rust/flow/output.rs"),
            statement = statement
        );

        assert_eq!(breathe(&source).unwrap(), expected);
    }
}

fn breathe(source: &str) -> Result<String, String> {
    super::breathe(source, &Rules::default())
}
