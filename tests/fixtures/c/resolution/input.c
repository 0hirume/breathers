int resolve(
    const int *from, int *expression, const int *limits
) {
    if (!from) {
        return 0;
    }
    return expression ? *from : 0;
}
