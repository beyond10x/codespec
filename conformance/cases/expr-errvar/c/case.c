/* The C source of the kit case expr-errvar, in runtime form: the sentinel error ErrEmpty, and
   the length of a cs_string compared with zero. */

#include "codespec_rt.h"

static struct cs_error_s ErrEmpty_s = {{"empty", 5}, NULL, NULL};
cs_error const ErrEmpty = &ErrEmpty_s;

/* codespec: func Check(string) error */
cs_error Check(cs_string s) {
    if (cs_string_eq(s, (cs_string){"", 0})) {
        return ErrEmpty;
    }
    return NULL;
}
