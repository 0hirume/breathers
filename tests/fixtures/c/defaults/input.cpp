struct Value { Value() = default; };
int example(Value parent = {}) {
    int first = 1; // trailing
    // attached
    if (first) {
        ++first;
    }
    return first;
}
