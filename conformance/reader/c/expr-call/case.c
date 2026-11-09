/* The C source of the kit case expr-call. */

/* Doubles. */
static long twice(long n) {
    return n + n;
}

/* Quadruples by calling twice twice. */
long quad(long n) {
    return twice(twice(n));
}
