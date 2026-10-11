int check(void);
void work(void);
void example(void) {
    int ready = check();
    if (ready) {
        work();
    }
}
