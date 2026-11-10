/* The C source of the kit case expr-to-error, in runtime form, as the C writer writes it: a
   value of a type T of the module as an error is cs_error_of with a function that gives the
   message of the value it points to by calling T_Error, the method Error of T, and a copy of the
   value. */

#include "codespec_rt.h"

typedef struct NotFound NotFound;

/* NotFound is an error naming a key. */
struct NotFound {
    cs_string Key;
};

cs_string NotFound_Error(NotFound e) {
    return cs_string_cat(((cs_string){"not found: ", 11}), e.Key);
}

static cs_string cs_message_NNotFound(const void *self) {
    return NotFound_Error(*(const NotFound *)self);
}

static cs_error cs_to_error_NNotFound(NotFound v) {
    return cs_error_of(cs_message_NNotFound, cs_dup(&v, sizeof v));
}

/* codespec: func Get(string) error */
cs_error Get(cs_string k) {
    return cs_to_error_NNotFound(((NotFound){.Key = k}));
}
