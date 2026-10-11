struct Example {
    int resolve(const int *value);
};

int
Example::resolve(const int *value) {
    if (!value) {
        return 0;
    }

    return *value;
}
