void work(void), process(void), finish(void);
int ready(void);

void example(void) {
    work();

    while (ready()) {
        process();
    }

    finish();
}
