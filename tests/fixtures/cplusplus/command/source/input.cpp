#include <value.hpp>
int example(Value parent = {}) {
    int first = 1;
    CHANGE(first);
    return first;
}
