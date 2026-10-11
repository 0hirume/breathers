struct Sources {
    int resolve(
        int *from, int *expression, int &) {
        if (!from) {
            return 0;
        }
        return expression ? *from : 0;
    }
};
