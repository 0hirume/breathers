use std::{collections::BTreeSet, iter, ops::Range};

pub(crate) fn apply(source: &str, insertions: BTreeSet<usize>) -> String {
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

pub(crate) fn boundary(
    source: &str,
    start: usize,
    end: usize,
    comments: impl Iterator<Item = Range<usize>>,
) -> Option<usize> {
    let mut position = start;
    let mut boundary = None;

    for (stop, next) in comments
        .map(|range| (range.start, range.end))
        .chain(iter::once((end, end)))
    {
        let gap = &source[position..stop];

        if gap.bytes().filter(|byte| *byte == b'\n').count() > 1 {
            return None;
        }

        if let Some(newline) = gap.find('\n') {
            if gap[..newline].trim_end_matches('\r').ends_with('\\') {
                return None;
            }

            boundary.get_or_insert(position + newline + 1);
        }

        position = next;
    }

    boundary
}
