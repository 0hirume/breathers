#ifndef VALUE_H
#define VALUE_H
#ifdef __cplusplus
extern "C" {
#endif
#ifndef ENABLED
#error missing compiler flag
#endif
#define FIELD(type) type field;
#define CHANGE(value) do { if (value) { ++value; } } while (0)
struct Value { FIELD(int) };
#ifdef __cplusplus
}
#endif
#endif
