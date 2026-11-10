/* The C source of the kit case op-eq, in runtime form: a string is the runtime's cs_string, and
   two compare equal through cs_string_eq. */

#include "codespec_rt.h"

/* codespec: func Eq(string, string) bool */
bool Eq(cs_string a, cs_string b) {
    return cs_string_eq(a, b);
}
