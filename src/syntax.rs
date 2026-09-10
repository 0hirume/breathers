use tree_sitter::Node;

pub fn multiline(node: Node<'_>, source: &str, opaque: fn(Node<'_>) -> bool) -> bool {
    if opaque(node) {
        return false;
    }
    let mut cursor = node.walk();
    let mut previous = node.start_byte();
    for child in node.children(&mut cursor) {
        if source[previous..child.start_byte()].contains('\n') || multiline(child, source, opaque) {
            return true;
        }
        previous = child.end_byte();
    }
    node.child_count() > 0 && source[previous..node.end_byte()].contains('\n')
}

pub fn boundary(source: &str, start: usize, end: usize, comments: &[Node<'_>]) -> Option<usize> {
    let mut position = start;
    let mut boundary = None;
    for (stop, next) in comments
        .iter()
        .map(|node| (node.start_byte(), node.end_byte()))
        .chain(std::iter::once((end, end)))
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
