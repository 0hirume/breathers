use crate::configuration::Rules;

#[test]
fn spaces_blocks_pipelines_and_returns() {
    let source = r"def example [] {
    let ready = true
    # attached
    if $ready {
        print ready
    }
    ls
    | where size > 0
    | get name
    return done
}
";

    let expected = r"def example [] {
    let ready = true

    # attached
    if $ready {
        print ready
    }

    ls
    | where size > 0
    | get name

    return done
}
";

    for (source, expected) in [
        (source.to_owned(), expected.to_owned()),
        (expected.to_owned(), expected.to_owned()),
        (source.replace('\n', "\r\n"), expected.replace('\n', "\r\n")),
    ] {
        assert_eq!(
            super::breathe(&source, &Rules::default()).unwrap(),
            expected
        );
    }

    super::breathe("def broken [] {", &Rules::default()).unwrap_err();
}

#[test]
fn preserves_literals_compact_code_and_pipeline_stages() {
    for source in [
        r"let text = 'first
second'
print done
",
        r"let text = r#'first
second'#
print done
",
        r#"let text = $"first
($value)"
print done
"#,
        r"let first = 1
let second = 2
",
        r"let ready = true; if $ready { print yes }; print done
",
        r"ls
| where size > 0
| get name
",
        r"#!/usr/bin/env nu
if true { print yes }
",
        r"print first


return done",
        r"print first
^return done
",
    ] {
        assert_eq!(super::breathe(source, &Rules::default()).unwrap(), source);
    }
}

#[test]
fn spaces_definitions_loops_and_modules() {
    for (source, expected) in [
        (
            r"print first
def example [] { print yes }
print last
",
            r"print first

def example [] { print yes }

print last
",
        ),
        (
            r"let values = [1 2]
for value in $values { print $value }
print last
",
            r"let values = [1 2]

for value in $values { print $value }

print last
",
        ),
        (
            r"mut ready = true
while $ready { $ready = false }
loop { break }
",
            r"mut ready = true

while $ready { $ready = false }

loop { break }
",
        ),
        (
            r"print first
try { print yes } catch { print error }
print last
",
            r"print first

try { print yes } catch { print error }

print last
",
        ),
        (
            r"module example {
    export def ready [] { true }
}
use example ready
",
            r"module example {
    export def ready [] { true }
}

use example ready
",
        ),
    ] {
        let formatted = super::breathe(source, &Rules::default()).unwrap();
        assert_eq!(formatted, expected);

        assert_eq!(
            super::breathe(expected, &Rules::default()).unwrap(),
            expected
        );
    }
}

#[test]
fn spaces_records_match_arms_and_closures() {
    for (source, expected) in [
        (
            r"let value = {
    first: 1
    second: [
        2
    ]
    third: 3
}
print $value
",
            r"let value = {
    first: 1

    second: [
        2
    ]

    third: 3
}

print $value
",
        ),
        (
            r"match $value {
    1 => {
        print first
    }
    _ => { print other }
}
",
            r"match $value {
    1 => {
        print first
    }

    _ => { print other }
}
",
        ),
        (
            r"let transform = { |value|
    print $value
    return $value
}
print done
",
            r"let transform = { |value|
    print $value

    return $value
}

print done
",
        ),
    ] {
        let formatted = super::breathe(source, &Rules::default()).unwrap();
        assert_eq!(formatted, expected);

        assert_eq!(
            super::breathe(expected, &Rules::default()).unwrap(),
            expected
        );
    }
}

#[test]
fn related_distinguishes_reads_from_writes_and_literals() {
    let rules: Rules = toml::from_str("related = true").unwrap();

    for source in [
        r"let values = [1 2]
for value in $values { print $value }
",
        r"let value = { name: ready }
return $value.name
",
        r#"let value = 1
return $"($value)"
"#,
        r"mut value = 1
$value += 2
return $value
",
        r"let value = 1
let result = (
    $value + 1
)
",
    ] {
        assert_eq!(super::breathe(source, &rules).unwrap(), source);
    }

    for (source, expected) in [
        (
            r"let value = 1
return value
",
            r"let value = 1

return value
",
        ),
        (
            r"let value = 1
return 'value'
",
            r"let value = 1

return 'value'
",
        ),
        (
            r"let value = 1
return { value: 2 }
",
            r"let value = 1

return { value: 2 }
",
        ),
        (
            r"let value = 1
return {|| $value }
",
            r"let value = 1

return {|| $value }
",
        ),
        (
            r"let value = 1
return $other.value
",
            r"let value = 1

return $other.value
",
        ),
    ] {
        assert_eq!(super::breathe(source, &rules).unwrap(), expected);
    }
}

#[test]
fn applies_granular_rules_and_related_dependencies() {
    for (source, configuration, expected) in [
        (
            r"let ready = true
if $ready { print yes }
",
            "related = true",
            r"let ready = true
if $ready { print yes }
",
        ),
        (
            r"let ready = true

if $ready { print yes }
",
            "related = true",
            r"let ready = true

if $ready { print yes }
",
        ),
        (
            r"let ready = true
if true { print $ready }
",
            "related = true",
            r"let ready = true

if true { print $ready }
",
        ),
        (
            r"print first
if true { print yes }
print last
",
            "blocks = false",
            r"print first
if true { print yes }
print last
",
        ),
        (
            r"print first
if true { print yes }
",
            r"blocks = false
conditionals = true",
            r"print first

if true { print yes }
",
        ),
        (
            r"print first
ls
| get name
print last
",
            "pipelines = false",
            r"print first
ls
| get name
print last
",
        ),
        (
            r"print first
return done
",
            "returns = false",
            r"print first
return done
",
        ),
    ] {
        let rules: Rules = toml::from_str(configuration).unwrap();
        assert_eq!(super::breathe(source, &rules).unwrap(), expected);
    }
}
