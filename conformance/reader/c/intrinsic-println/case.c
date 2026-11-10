/* The C source of the kit case intrinsic-println, in runtime form: a printed line is a string
   builder from cs_sb_new, its values separated by one space, printed by cs_sb_println. */

#include "codespec_rt.h"

/* codespec: func Show(string, int) */
void Show(cs_string name, int64_t n) {
    cs_sb_println(cs_fmt_int(cs_sb_bytes(cs_fmt_str(cs_sb_new(), name), " ", 1), n));
}
