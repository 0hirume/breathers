struct Object { int ready; };
extern struct Object *object;
void work(void);
void example(void) {
    int ready = 1;
    if (object->ready) {
        work();
    }
}
