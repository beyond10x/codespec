/* The C source of the kit case unop-deref. */

/* Writes a local through a pointer to it. */
long through(long n) {
    long x = 0;
    long *p = &x;
    *p = n;
    return x;
}
