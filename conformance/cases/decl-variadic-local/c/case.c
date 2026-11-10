/* The C source of the kit case decl-variadic-local, in runtime form: a variadic parameter of
   element type T is an int64_t count and `...`, and the body begins by appending each of that
   many arguments, read with va_arg as T, to the slice the parameter is. A call passes the count
   first. C leaves the order of an operator's calls unspecified, so each call is a statement. */

#include <stdarg.h>

#include "codespec_rt.h"

/* codespec: func sum(...int) int */
static int64_t sum(int64_t cs_n, ...) {
    cs_slice xs = {0};
    va_list cs_ap;
    va_start(cs_ap, cs_n);
    for (int64_t cs_i = 0; cs_i < cs_n; cs_i++) {
        int64_t cs_v = va_arg(cs_ap, int64_t);
        xs = cs_append(xs, &cs_v, sizeof(int64_t));
    }
    va_end(cs_ap);
    int64_t t = 0;
    for (int64_t i = 0; i < xs.len; i++) {
        int64_t x = *(int64_t *)cs_slice_at(xs, i, sizeof(int64_t));
        t = (int64_t)((uint64_t)t + (uint64_t)x);
    }
    return t;
}

/* codespec: func Sums(int, int) int */
int64_t Sums(int64_t a, int64_t b) {
    int64_t none = sum(0);
    int64_t one = sum(1, a);
    int64_t three = sum(3, a, b, (int64_t)3);
    return (int64_t)((uint64_t)(int64_t)((uint64_t)none + (uint64_t)one) + (uint64_t)three);
}
