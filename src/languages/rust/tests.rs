use crate::configuration::{Configuration, Rules};

#[test]
fn spaces_multiline_items() {
    let source = r"pub(crate) use options::Options;
pub(super) struct Layout<'a> {
    pub(crate) source: &'a str,
    pub(crate) options: &'a Options,
}
use other::Other;
";
    let expected = r"pub(crate) use options::Options;

pub(super) struct Layout<'a> {
    pub(crate) source: &'a str,
    pub(crate) options: &'a Options,
}

use other::Other;
";
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
fn spaces_function_items_in_files_modules_and_implementations() {
    let functions = r"pub(super) fn first() -> u8 {
    1
}
/// Second function.
#[inline]
pub(super) fn second() -> u8 {
    2
}
";
    let expected = r"pub(super) fn first() -> u8 {
    1
}

/// Second function.
#[inline]
pub(super) fn second() -> u8 {
    2
}
";
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
    let source = r"fn example() {
    let first = 1;
    let second = 2; // trailing
    // attached
    if first < second {
        work();
    }
    let values = [
        first,
        second,
    ];
    finish();
}
";
    let expected = r"fn example() {
    let first = 1;
    let second = 2; // trailing

    // attached
    if first < second {
        work();
    }

    let values = [
        first,
        second,
    ];

    finish();
}
";
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
        r##"fn example() {
    let text = r#"first
second"#;
    finish();
}
"##,
        r"macro_rules! example { () => { first(); if ready() { second(); } third(); }; }
",
        r"fn example() {
    first();

    // attached
    if ready() {}
}
",
    ] {
        assert_eq!(breathe(source).unwrap(), source);
    }

    breathe("fn broken( {").unwrap_err();
}

#[test]
fn spaces_multiline_struct_fields() {
    let source = r"struct Processor {
    name: String,
    enabled: bool,
    callback: Box<
        dyn Fn(&Request) -> Result<Response, Error>
            + Send
            + Sync,
    >,
    retries: usize,
}
";
    let expected = r"struct Processor {
    name: String,
    enabled: bool,

    callback: Box<
        dyn Fn(&Request) -> Result<Response, Error>
            + Send
            + Sync,
    >,

    retries: usize,
}
";
    assert_eq!(breathe(source).unwrap(), expected);
    assert_eq!(breathe(expected).unwrap(), expected);

    assert_eq!(
        breathe(&source.replace('\n', "\r\n")).unwrap(),
        expected.replace('\n', "\r\n")
    );

    let source = r"struct Tuple(
    u32,
    Box<
        String,
    >,
    bool,
);
";
    let expected = r"struct Tuple(
    u32,

    Box<
        String,
    >,

    bool,
);
";
    assert_eq!(breathe(source).unwrap(), expected);

    for source in [
        r"struct Simple {
    first: u32,
    second: bool,
}
",
        r"struct Only {
    value: Box<
        String,
    >,
}
",
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
            r"fn example() {{
    let result = calculate();
    save();
    {expression}
}}
"
        );
        let expected = format!(
            r"fn example() {{
    let result = calculate();
    save();

    {expression}
}}
"
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
        r"fn example() {
    first();
    let callback = || return result;
    finish();
}
",
    ] {
        assert_eq!(breathe(source).unwrap(), source);
    }
}

#[test]
fn spaces_multiline_enum_variants() {
    let source = r"enum Example {
    First,
    Second(u32), // trailing
    // attached
    Record {
        value: u32,
    },
    Tuple(
        u32,
        String,
    ),
    Last,
}
";
    let expected = r"enum Example {
    First,
    Second(u32), // trailing

    // attached
    Record {
        value: u32,
    },

    Tuple(
        u32,
        String,
    ),

    Last,
}
";
    assert_eq!(breathe(source).unwrap(), expected);
    assert_eq!(breathe(expected).unwrap(), expected);

    assert_eq!(
        breathe(&source.replace('\n', "\r\n")).unwrap(),
        expected.replace('\n', "\r\n")
    );

    for source in [
        r"enum Example { First, Second(u32), Record { value: u32 } }
",
        r"enum Example {
    First,
    Second(u32),
    Record { value: u32 },
}
",
        r"enum Example {
    Record {
        value: u32,
    },
}
",
    ] {
        assert_eq!(breathe(source).unwrap(), source);
    }
}

#[test]
fn spaces_multiline_match_arms() {
    let source = r"fn example() {
    match kind {
        Kind::First => 1,
        Kind::Second => 2, // trailing
        // attached
        Kind::Index => Value::Index {
            receiver: children.next()?,
            key: children.next()?,
        },
        Kind::Instantiate => Value::Instantiate {
            expression: children.next()?,
            arguments: children.next()?,
        },
        Kind::Last => 3,
    }
}
";
    let expected = r"fn example() {
    match kind {
        Kind::First => 1,
        Kind::Second => 2, // trailing

        // attached
        Kind::Index => Value::Index {
            receiver: children.next()?,
            key: children.next()?,
        },

        Kind::Instantiate => Value::Instantiate {
            expression: children.next()?,
            arguments: children.next()?,
        },

        Kind::Last => 3,
    }
}
";
    assert_eq!(breathe(source).unwrap(), expected);
    assert_eq!(breathe(expected).unwrap(), expected);

    assert_eq!(
        breathe(&source.replace('\n', "\r\n")).unwrap(),
        expected.replace('\n', "\r\n")
    );

    for source in [
        r"fn example() {
    match kind {
        First => if ready() { 1 } else { 2 },
        Second => { 3 },
        _ => 4,
    }
}
",
        r##"fn example() {
    match kind {
        First => r#"first
second"#,
        _ => "last",
    }
}
"##,
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
            r"fn example() {{
    first();
    {statement}
    result
}}
"
        );
        let expected = format!(
            r"fn example() {{
    first();

    {statement}

    result
}}
"
        );
        assert_eq!(breathe(&source).unwrap(), expected);
    }
}

fn breathe(source: &str) -> Result<String, String> {
    super::breathe(source, &Rules::default())
}
