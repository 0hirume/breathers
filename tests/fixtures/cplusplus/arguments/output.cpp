void example() {
    MACRO([] {
        first();
        return second();
    });

    finish();
}
