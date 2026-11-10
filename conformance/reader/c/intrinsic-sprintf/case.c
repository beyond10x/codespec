/* The C source of the kit case intrinsic-sprintf, in runtime form: a formatted string is a string
   builder from cs_sb_new, its pieces appended in order, made a string by cs_sb_string. */

#include "codespec_rt.h"

/* codespec: func Label(string, int) string */
cs_string Label(cs_string name, int64_t n) {
    return cs_sb_string(cs_fmt_int(cs_sb_bytes(cs_fmt_str(cs_sb_new(), name), "=", 1), n));
}
