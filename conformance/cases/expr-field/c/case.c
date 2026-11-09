/* The C source of the kit case expr-field. */

/* Two values. */
struct Pair {
    long a;
    long b;
};

/* Returns the second field. */
long right(long a, long b) {
    struct Pair p = {.a = a, .b = b};
    return p.b;
}
