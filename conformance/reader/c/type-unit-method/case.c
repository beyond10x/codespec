/* The C source of the kit case type-unit-method. */

/* A counter. */
struct Counter {
    long n;
};

/* Adds one to the counter c points to and returns nothing. */
static void bump(struct Counter *c) {
    c->n++;
}

/* Bumps a counter k times through a function that returns nothing. */
long count(long k) {
    struct Counter c = {0};
    for (long i = 0; i < k; i++) {
        bump(&c);
    }
    return c.n;
}
