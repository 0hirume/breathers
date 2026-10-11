use crate::configuration::Rules;

fn breathe(source: &str) -> Result<String, String> {
    super::breathe(source, &Rules::default())
}

#[test]
fn spaces_blocks_calls_tables_and_returns() {
    let source = include_str!("../../../tests/fixtures/lua/spacing/input.lua");

    let expected = include_str!("../../../tests/fixtures/lua/spacing/output.lua");

    assert_eq!(breathe(source).unwrap(), expected);
    assert_eq!(breathe(expected).unwrap(), expected);

    assert_eq!(
        breathe(&source.replace('\n', "\r\n")).unwrap(),
        expected.replace('\n', "\r\n")
    );

    breathe("local broken = {").unwrap_err();
    breathe("local value: number = 1").unwrap_err();
}

#[test]
fn supports_lua_syntax_and_nested_functions() {
    let source = include_str!("../../../tests/fixtures/lua/syntax/input.lua");

    let expected = include_str!("../../../tests/fixtures/lua/syntax/output.lua");

    assert_eq!(breathe(source).unwrap(), expected);

    let source = include_str!("../../../tests/fixtures/lua/function/input.lua");

    let expected = include_str!("../../../tests/fixtures/lua/function/output.lua");

    assert_eq!(breathe(source).unwrap(), expected);
}

#[test]
fn preserves_strings_comments_and_compact_code() {
    for source in [
        r"local text = [=[first
second]=]
work()
",
        r"local value = 1 --[[first
second]]
work()
",
        r"local first = 1; if first > 0 then work() end; finish()
",
        r"#!/usr/bin/env lua
local first = 1
local second = 2
",
        r"local first = 1

if first > 0 then
    work()
end
",
    ] {
        assert_eq!(breathe(source).unwrap(), source);
    }
}
