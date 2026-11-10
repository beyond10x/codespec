/* The C source of the kit case type-func. */

static long twice(long n) {
    return n * 2;
}

static long run(long (*f)(long), long n) {
    return f(n);
}

/* Passes twice to run, whose parameter is a function pointer. */
long apply(long n) {
    return run(twice, n);
}
