enum Example {
    First,
    Second(u32), // trailing

    // attached
    Record {
        value: u32,
    },

    Tuple(
        u32,
        String,
    ),

    Last,
}
