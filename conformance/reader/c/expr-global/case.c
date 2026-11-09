/* The C source of the kit case expr-global. */

static long total;

/* Adds n to the running total and returns the new total. */
long add(long n) {
    total = total + n;
    return total;
}
