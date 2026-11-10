/* The C source of the kit case stmt-continue. */

/* Sums the values up to n-1 except skip. */
long skipSum(long n, long skip) {
    long s = 0;
    for (long x = 0; x < n; x++) {
        if (x == skip) {
            continue;
        }
        s += x;
    }
    return s;
}
