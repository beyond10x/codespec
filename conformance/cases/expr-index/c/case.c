/* The C source of the kit case expr-index, in runtime form: an element of a slice is read
   through cs_slice_at, cast to a pointer to the element type. */

#include "codespec_rt.h"

/* codespec: func At(int) int */
int64_t At(int64_t i) {
    cs_slice xs = cs_slice_make();
    xs = cs_append(xs, &(int64_t){10}, sizeof(int64_t));
    xs = cs_append(xs, &(int64_t){20}, sizeof(int64_t));
    return *(int64_t *)cs_slice_at(xs, i, sizeof(int64_t));
}
