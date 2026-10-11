fn example() {
    match kind {
        Kind::First => 1,
        Kind::Second => 2, // trailing
        // attached
        Kind::Index => Value::Index {
            receiver: children.next()?,
            key: children.next()?,
        },
        Kind::Instantiate => Value::Instantiate {
            expression: children.next()?,
            arguments: children.next()?,
        },
        Kind::Last => 3,
    }
}
