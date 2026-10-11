use std::slice::from_ref;

use crate::configuration::Rules;

fn breathe(source: &str) -> Result<String, String> {
    super::breathe(source, &Rules::default(), &[])
}

#[test]
fn spaces_cplusplus_statements() {
    let source = r"int example(void) {
    int first = 1;
    int second = 2; // trailing
    // attached
    if (first < second) {
        work();
    }
    save();
    return first;
}
";

    let expected = r"int example(void) {
    int first = 1;
    int second = 2; // trailing

    // attached
    if (first < second) {
        work();
    }

    save();

    return first;
}
";

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
            r"void example() {
    MACRO([] {
        first();
        return second();
    });
    finish();
}
",
            r"void example() {
    MACRO([] {
        first();
        return second();
    });

    finish();
}
",
        ),
        (
            r#"void example() {
    auto text = R"(first
second)";
    finish();
}
"#,
            r#"void example() {
    auto text = R"(first
second)";
    finish();
}
"#,
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
    let source = r"#ifndef EXAMPLE_H
#define EXAMPLE_H
int example(void) {
    work();
    return 1;
}
#endif
";

    assert_eq!(
        breathe(source).unwrap(),
        r"#ifndef EXAMPLE_H
#define EXAMPLE_H
int example(void) {
    work();

    return 1;
}
#endif
"
    );

    let source = r"struct Example {
    int first;
    int (*callback)(
        int,
        int
    );
    int last;
};
";

    let expected = r"struct Example {
    int first;

    int (*callback)(
        int,
        int
    );

    int last;
};
";

    assert_eq!(breathe(source).unwrap(), expected);

    let source = r"void example(int value) {
    switch (value) {
        case 1:
            work();
            break;
        default:
            finish();
            break;
    }
}
";
    let expected = r"void example(int value) {
    switch (value) {
        case 1:
            work();
            break;

        default:
            finish();
            break;
    }
}
";

    assert_eq!(breathe(source).unwrap(), expected);
}

#[test]
fn protects_inactive_code_without_pruning_active_code() {
    let source = r"#if 0
void inactive() {
    work();
    return 1;
}
#endif
void active() {
    work();
    return 1;
}
";
    let expected = r"#if 0
void inactive() {
    work();
    return 1;
}
#endif
void active() {
    work();

    return 1;
}
";
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
