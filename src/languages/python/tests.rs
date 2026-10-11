use crate::configuration::Rules;

#[test]
fn spaces_python_without_touching_indentation_or_strings() {
    let source = include_str!("../../../tests/fixtures/python/spacing/input.py");

    let expected = include_str!("../../../tests/fixtures/python/spacing/output.py");

    let rules = Rules::default();
    assert_eq!(super::breathe(source, &rules).unwrap(), expected);
    assert_eq!(super::breathe(expected, &rules).unwrap(), expected);

    assert_eq!(
        super::breathe(&source.replace('\n', "\r\n"), &rules).unwrap(),
        expected.replace('\n', "\r\n")
    );

    for source in [
        r"text = '''first
second'''
work()
",
        r"text = f'''first {value}
second'''
work()
",
        r"def example(): return 1
",
    ] {
        assert_eq!(super::breathe(source, &rules).unwrap(), source);
    }

    super::breathe(
        r"def broken(:
",
        &rules,
    )
    .unwrap_err();
}

#[test]
fn preserves_function_docstring_boundaries() {
    let rules = Rules::default();

    for statement in [
        r"return 1
",
        r"if ready:
        work()
",
        r"save(
        value,
    )
",
        r"values = [
        1,
    ]
",
    ] {
        let source = format!(
            r#"def example():
    """Describe the result."""
    {statement}"#
        );

        assert_eq!(super::breathe(&source, &rules).unwrap(), source);
    }

    let source = include_str!("../../../tests/fixtures/python/documentation/input.py");

    let expected = include_str!("../../../tests/fixtures/python/documentation/output.py");

    assert_eq!(super::breathe(source, &rules).unwrap(), expected);
}

#[test]
fn preserves_import_groups() {
    let rules = Rules::default();

    for source in [
        r"import example
from other import (
    first,
)
import another
",
        include_str!("../../../tests/fixtures/python/imports/input.py"),
        r"from __future__ import annotations
from __future__ import (
    generator_stop,
)
",
        include_str!("../../../tests/fixtures/python/grouping/input.py"),
    ] {
        assert_eq!(super::breathe(source, &rules).unwrap(), source);
    }

    let source = r"from example import (
    first,
)
value = 1
";
    let expected = r"from example import (
    first,
)

value = 1
";

    assert_eq!(super::breathe(source, &rules).unwrap(), expected);
}

#[test]
fn disables_blocks_without_multiline_overriding_the_toggle() {
    let source = r"first = 1
if first:
    work()
finish()
";
    let rules: Rules = toml::from_str("blocks = false").unwrap();
    assert_eq!(super::breathe(source, &rules).unwrap(), source);
}
