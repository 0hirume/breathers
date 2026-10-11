#include <value.hpp>
int example(Value parent = {}) {
    int second = 1;
    CHANGE(second);

    return second;
}
