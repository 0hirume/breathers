use std::slice::from_ref;

use crate::configuration::Rules;

fn breathe(source: &str) -> Result<String, String> {
    super::breathe(source, &Rules::default(), &[])
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
    let end = source.find("void active").unwrap();
    let protected = 0..end;
    let rules = Rules::default();

    assert_eq!(
        super::breathe(source, &rules, from_ref(&protected)).unwrap(),
        expected
    );

    assert_eq!(
        super::breathe(expected, &rules, from_ref(&protected)).unwrap(),
        expected
    );
}
