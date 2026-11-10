/* The C source of the kit case expr-func-value. C has no method value: the step k is added
   after the function value is called. */

static long twice(long n) {
    return n * 2;
}

/* Applies the function twice, used as a value, to n and adds k. */
long compose(long n, long k) {
    long (*f)(long) = twice;
    return f(n) + k;
}
