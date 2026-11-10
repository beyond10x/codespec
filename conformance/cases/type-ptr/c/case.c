/* The C source of the kit case type-ptr. */

/* Increments a value through a pointer. */
long bump(long n) {
    long *p = &n;
    *p = *p + 1;
    return n;
}
