/* The C source of the kit case stmt-break. */

/* The smallest i with i*i > n. */
long root(long n) {
    long i = 0;
    for (;;) {
        if (i * i > n) {
            break;
        }
        i++;
    }
    return i;
}
