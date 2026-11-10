/* The C source of the kit case intrinsic-len, in runtime form: a slice is the runtime's cs_slice,
   its element type the one cs_append is given, and its length the field len. */

#include "codespec_rt.h"

/* codespec: func Count(int) int */
int64_t Count(int64_t n) {
    cs_slice xs = cs_slice_make();
    for (int64_t i = 0; i < n; i++) {
        xs = cs_append(xs, &(cs_string){"x", 1}, sizeof(cs_string));
    }
    return xs.len;
}
