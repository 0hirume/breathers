use similar::TextDiff;

pub(super) fn diff(path: &str, source: &str, formatted: &str) -> String {
    let path = if cfg!(windows) {
        path.replace('\\', "/")
    } else {
        path.to_owned()
    };

    TextDiff::from_lines(source, formatted)
        .unified_diff()
        .header(&label(&format!("a/{path}")), &label(&format!("b/{path}")))
        .to_string()
}

fn label(path: &str) -> String {
    if !path
        .chars()
        .any(|character| character.is_ascii_control() || matches!(character, '"' | '\\'))
    {
        return path.to_owned();
    }

    let mut output = String::from("\"");

    for character in path.chars() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),

            character if character.is_ascii_control() => {
                let escaped = format!("\\{:03o}", u32::from(character));
                output.push_str(&escaped);
            }

            character => output.push(character),
        }
    }

    output.push('"');

    output
}
