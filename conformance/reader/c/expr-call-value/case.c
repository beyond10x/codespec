/* The C source of the kit case expr-call-value. */

#include <stdbool.h>

/* Holds a check as a function pointer. */
struct Rule {
    bool (*check)(long);
};

static bool positive(long n) {
    return n > 0;
}

/* Calls the function a rule holds in a field on n. */
bool allowed(long n) {
    struct Rule r = {positive};
    return r.check(n);
}
