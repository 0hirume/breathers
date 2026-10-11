int example(void) {
    int first = 1;
    int second = 2; // trailing

    // attached
    if (first < second) {
        work();
    }

    save();

    return first;
}
