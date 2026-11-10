/* The C source of the kit case stmt-for-range, in runtime form: a loop over a slice counts its
   index up to the slice's length and reads each element into a variable first. */

#include "codespec_rt.h"

/* codespec: func Total(int) int */
int64_t Total(int64_t n) {
    cs_slice xs = cs_slice_make();
    for (int64_t i = 0; i < n; i++) {
        xs = cs_append(xs, &(int64_t){i}, sizeof(int64_t));
    }
    int64_t s = 0;
    for (int64_t j = 0; j < xs.len; j++) {
        int64_t x = *(int64_t *)cs_slice_at(xs, j, sizeof(int64_t));
        s = (int64_t)((uint64_t)s + (uint64_t)x);
    }
    return s;
}
