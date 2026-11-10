/* The C source of the kit case sem-error-propagate, in runtime form: an error result is a
   cs_error, NULL when nil; a function of a value and an error returns <name>_result. */

#include "codespec_rt.h"

static struct cs_error_s ErrNegative_s = {{"negative", 8}, NULL, NULL};
cs_error const ErrNegative = &ErrNegative_s;

typedef struct {
    int64_t r0;
    cs_error r1;
} Checked_result;

/* codespec: func Checked(int) (int, error) */
Checked_result Checked(int64_t n) {
    if (n < 0) {
        return (Checked_result){0, ErrNegative};
    }
    return (Checked_result){n, NULL};
}

/* codespec: func Valid(int) error */
cs_error Valid(int64_t n) {
    if (n < 0) {
        return ErrNegative;
    }
    return NULL;
}

typedef struct {
    int64_t r0;
    cs_error r1;
} Through_result;

/* codespec: func Through(int) (int, error) */
Through_result Through(int64_t n) {
    Checked_result cs_t1 = Checked(n);
    int64_t v = cs_t1.r0;
    cs_error err = cs_t1.r1;
    if (err != NULL) {
        return (Through_result){0, err};
    }
    return (Through_result){v, NULL};
}

typedef struct {
    int64_t r0;
    cs_error r1;
} Guarded_result;

/* codespec: func Guarded(int) (int, error) */
Guarded_result Guarded(int64_t n) {
    cs_error err = Valid(n);
    if (err != NULL) {
        return (Guarded_result){0, err};
    }
    return (Guarded_result){n, NULL};
}
