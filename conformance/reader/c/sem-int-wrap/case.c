/* The C source of the kit case sem-int-wrap, in runtime form: a signed +, * and negation go
   through uint64_t and back, which wraps; / and % through the runtime's cs_div and cs_rem, which
   give MinInt / -1 and MinInt % -1 as Go does. */

#include "codespec_rt.h"

/* codespec: func Add(int, int) int */
int64_t Add(int64_t a, int64_t b) {
    return (int64_t)((uint64_t)a + (uint64_t)b);
}

/* codespec: func Mul(int, int) int */
int64_t Mul(int64_t a, int64_t b) {
    return (int64_t)((uint64_t)a * (uint64_t)b);
}

/* codespec: func Div(int, int) int */
int64_t Div(int64_t a, int64_t b) {
    return cs_div_int64_t(a, b);
}

/* codespec: func Rem(int, int) int */
int64_t Rem(int64_t a, int64_t b) {
    return cs_rem_int64_t(a, b);
}

/* codespec: func Neg(int) int */
int64_t Neg(int64_t a) {
    return (int64_t)((uint64_t)0 - (uint64_t)a);
}
