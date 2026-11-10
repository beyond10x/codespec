/* The C source of the kit case stmt-expr-method. */

/* A counter. */
struct Counter {
    long n;
};

/* Adds one to the counter c points to. */
static void bump(struct Counter *c) {
    c->n++;
}

/* Bumps a counter k times, each call a statement of its own. */
long count(long k) {
    struct Counter c = {0};
    for (long i = 0; i < k; i++) {
        bump(&c);
    }
    return c.n;
}
