/* The C source of the kit case expr-const. */

/* The largest value clamp returns. */
enum { Limit = 10 };

/* Caps n at Limit. */
int clamp(int n) {
    if (n > Limit) {
        return Limit;
    }
    return n;
}
