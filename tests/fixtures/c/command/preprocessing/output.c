#include <value.h>
int example(void) {
    struct Value value = { .field = ENABLED };
    int class = value.field;
    CHANGE(class);

    return class;
}
