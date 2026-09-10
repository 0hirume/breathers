use std::collections::BTreeSet;

pub fn apply(source: &str, insertions: BTreeSet<usize>) -> String {
    let mut output = String::new();
    let mut previous = 0;

    for offset in insertions {
        output.push_str(&source[previous..offset]);

        output.push_str(if source[..offset].ends_with("\r\n") {
            "\r\n"
        } else {
            "\n"
        });

        previous = offset;
    }

    output.push_str(&source[previous..]);

    output
}
