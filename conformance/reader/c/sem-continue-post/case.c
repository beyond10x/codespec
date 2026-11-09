/* The C source of the kit case sem-continue-post. */

/* Sums 0 up to n-1 but skip; the continue runs i++ first. */
long skipOne(long n, long skip) {
    long s = 0;
    for (long i = 0; i < n; i++) {
        if (i == skip) {
            continue;
        }
        s += i;
    }
    return s;
}
