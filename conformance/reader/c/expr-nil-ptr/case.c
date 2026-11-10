/* The C source of the kit case expr-nil-ptr. */

#include <stdbool.h>

/* Points p at n when n is positive, else leaves it null; tells whether p is set. */
bool found(long n) {
    long *p = 0;
    if (n > 0) {
        p = &n;
    }
    return p != 0;
}
