/* The C source of the kit case type-any, in runtime form: an any is a cs_any, made by cs_box from
   the address of a value and the cs_type that describes its type. */

#include "codespec_rt.h"

static const cs_type type_int = {CS_K_INT64, "int", NULL};

/* codespec: func Boxed(int) any */
cs_any Boxed(int64_t n) {
    return cs_box(&type_int, &n, sizeof(int64_t));
}
