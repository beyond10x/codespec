/* The C source of the kit case stmt-for. */

/* Sums 0 up to n-1. */
long below(long n) {
    long s = 0;
    for (long i = 0; i < n; i++) {
        s += i;
    }
    return s;
}
