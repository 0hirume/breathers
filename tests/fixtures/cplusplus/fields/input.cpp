struct Example {
    int first;
    int (*callback)(
        int,
        int
    );
    int last;
};
