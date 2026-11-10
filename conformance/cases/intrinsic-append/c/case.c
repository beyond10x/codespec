/* The C source of the kit case intrinsic-append, in runtime form: cs_append adds a copy of the
   loop counter to the slice. */

#include "codespec_rt.h"

/* codespec: func Build(int) []int */
cs_slice Build(int64_t n) {
    cs_slice xs = cs_slice_make();
    for (int64_t i = 0; i < n; i++) {
        xs = cs_append(xs, &(int64_t){i}, sizeof(int64_t));
    }
    return xs;
}
