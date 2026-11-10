/* The C source of the kit case type-tuple, in runtime form: a function of two results returns the
   struct <name>_result of fields r0 and r1, and a caller binds both from one temporary. */

#include "codespec_rt.h"

typedef struct {
    int64_t r0;
    int64_t r1;
} split_result;

/* codespec: func split(int, int) (int, int) */
static split_result split(int64_t a, int64_t b) {
    return (split_result){b, a};
}

typedef struct {
    int64_t r0;
    int64_t r1;
} Flip_result;

/* codespec: func Flip(int, int) (int, int) */
Flip_result Flip(int64_t a, int64_t b) {
    split_result cs_t1 = split(a, b);
    int64_t x = cs_t1.r0;
    int64_t y = cs_t1.r1;
    return (Flip_result){x, y};
}
