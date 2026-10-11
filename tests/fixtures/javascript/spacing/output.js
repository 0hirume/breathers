function example() {
    const first = 1;
    const second = 2; // trailing

    // attached
    if (first < second) {
        work();
    } else {
        other();
    }

    save();

    return first;
}
