#define IDENTITY(value) value
int example() {
    auto callback = IDENTITY([] {
        int first = 1;
        return first;
    });

    return callback();
}
