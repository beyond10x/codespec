/* The C source of the kit case intrinsic-errorf, in runtime form: a formatted error is a string
   builder from cs_sb_new, its pieces appended in order, made an error by cs_sb_error. */

#include "codespec_rt.h"

/* codespec: func Validate(int) error */
cs_error Validate(int64_t n) {
    if (n < 0) {
        return cs_sb_error(cs_fmt_int(cs_sb_bytes(cs_sb_new(), "negative: ", 10), n));
    }
    return NULL;
}
