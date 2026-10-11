fn example() {
    match kind {
        First => if ready() { 1 } else { 2 },
        Second => { 3 },
        _ => 4,
    }
}
