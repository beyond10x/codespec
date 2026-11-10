/* The C source of the kit case type-slice, in runtime form: an empty slice is cs_slice_make, and
   cs_append copies the value of a variable of the element type into it. */

#include "codespec_rt.h"

/* codespec: func Pair(string, string) []string */
cs_slice Pair(cs_string a, cs_string b) {
    cs_slice xs = cs_slice_make();
    xs = cs_append(xs, &a, sizeof(cs_string));
    xs = cs_append(xs, &b, sizeof(cs_string));
    return xs;
}
