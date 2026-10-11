pub(super) fn diff(path: &str, source: &str, formatted: &str) -> String {
    let path = if cfg!(windows) {
        path.replace('\\', "/")
    } else {
        path.to_owned()
    };

    let before = source.split_inclusive('\n').count();
    let after = formatted.split_inclusive('\n').count();
    let original = label(&format!("a/{path}"));
    let replacement = label(&format!("b/{path}"));

    let mut output = format!(
        "--- {original}\n+++ {replacement}\n@@ -{},{before} +{},{after} @@\n",
        usize::from(before > 0),
        usize::from(after > 0),
    );

    for (prefix, text) in [('-', source), ('+', formatted)] {
        for line in text.split_inclusive('\n') {
            output.push(prefix);
            output.push_str(line);

            if !line.ends_with('\n') {
                output.push_str("\n\\ No newline at end of file\n");
            }
        }
    }

    output
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
