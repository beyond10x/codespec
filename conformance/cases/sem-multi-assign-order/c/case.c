/* The C source of the kit case sem-multi-assign-order, in runtime form: an assignment of several
   values reads each into a temporary cs_t<n> before it assigns any. */

#include "codespec_rt.h"

typedef struct {
    int64_t r0;
    int64_t r1;
} Swap_result;

/* codespec: func Swap(int, int) (int, int) */
Swap_result Swap(int64_t a, int64_t b) {
    int64_t cs_t1 = b;
    int64_t cs_t2 = a;
    a = cs_t1;
    b = cs_t2;
    return (Swap_result){a, b};
}

/* codespec: func Fib(int) int */
int64_t Fib(int64_t n) {
    int64_t a = 0;
    int64_t b = 1;
    for (int64_t i = 0; i < n; i++) {
        int64_t cs_t1 = b;
        int64_t cs_t2 = (int64_t)((uint64_t)a + (uint64_t)b);
        a = cs_t1;
        b = cs_t2;
    }
    return a;
}
