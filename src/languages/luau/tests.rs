use crate::configuration::Rules;

fn breathe(source: &str) -> Result<String, String> {
    super::breathe(source, &Rules::default())
}

#[test]
fn spaces_blocks_multiline_values_and_returns() {
    let source = include_str!("../../../tests/fixtures/luau/spacing/input.luau");
    let expected = include_str!("../../../tests/fixtures/luau/spacing/output.luau");

    assert_eq!(breathe(source).unwrap(), expected);
    assert_eq!(breathe(expected).unwrap(), expected);

    assert_eq!(
        breathe(&source.replace('\n', "\r\n")).unwrap(),
        expected.replace('\n', "\r\n")
    );

    breathe(r"local value = {").unwrap_err();
}

#[test]
fn groups_services_imports_types_and_populated_declarations() {
    let source = include_str!("../../../tests/fixtures/luau/grouping/input.luau");
    let expected = include_str!("../../../tests/fixtures/luau/grouping/output.luau");

    assert_eq!(breathe(source).unwrap(), expected);
    assert_eq!(breathe(expected).unwrap(), expected);
}

#[test]
fn preserves_literals_comments_and_compact_code() {
    for source in [
        r"local text = [=[first
second]=]
work()
",
        r"local text = `first {value} second`
work()
",
        r"local value = 1 --[[first
second]]
work()
",
        r"local first = 1; if first > 0 then work() end; finish()
",
        r"local first = 1
local second = 2
",
        r"declare function first(): number
declare function second(): string
",
    ] {
        assert_eq!(breathe(source).unwrap(), source);
    }

    let source = include_str!("../../../tests/fixtures/luau/function/input.luau");
    let expected = include_str!("../../../tests/fixtures/luau/function/output.luau");

    assert_eq!(breathe(source).unwrap(), expected);
}
