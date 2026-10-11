#define BODY do { int first = 1; if (first) { ++first; } } while (0)
int example() {
    BODY;
    const char *text = R"(one
two)";
    return text[0];
}
