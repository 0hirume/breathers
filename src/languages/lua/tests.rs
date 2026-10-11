use crate::configuration::Rules;

fn breathe(source: &str) -> Result<String, String> {
    super::breathe(source, &Rules::default())
}

#[test]
fn spaces_blocks_calls_tables_and_returns() {
    let source = r"local first = 1
local second = 2 -- trailing
-- attached
if first < second then
    work()
elseif first == second then
    same()
else
    other()
end
local values = {
    first,
    second,
}
save(
    values
)
return values
";

    let expected = r"local first = 1
local second = 2 -- trailing

-- attached
if first < second then
    work()
elseif first == second then
    same()
else
    other()
end

local values = {
    first,
    second,
}

save(
    values
)

return values
";

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
    let source = r"local value <const> = 8 // 2
::again::
value = value & 3
if value > 0 then
    goto again
end
return value
";

    let expected = r"local value <const> = 8 // 2
::again::
value = value & 3

if value > 0 then
    goto again
end

return value
";

    assert_eq!(breathe(source).unwrap(), expected);

    let source = r"local function example()
    work()
    return 1
end
local value = example()
";

    let expected = r"local function example()
    work()

    return 1
end

local value = example()
";

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
