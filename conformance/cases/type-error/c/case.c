/* The C source of the kit case type-error, in runtime form: an error is the runtime's cs_error,
   nil is NULL, and a sentinel error a constant pointer to a static cs_error_s holding its
   message. */

#include "codespec_rt.h"

static struct cs_error_s ErrBad_s = {{"bad", 3}, NULL, NULL};
cs_error const ErrBad = &ErrBad_s;

/* codespec: func Fail(bool) error */
cs_error Fail(bool bad) {
    if (bad) {
        return ErrBad;
    }
    return NULL;
}
