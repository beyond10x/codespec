/* The C source of the kit case expr-intrinsic, in runtime form: the length of a cs_string is its
   field n. */

#include "codespec_rt.h"

/* codespec: func Size(string) int */
int64_t Size(cs_string s) {
    return s.n;
}
