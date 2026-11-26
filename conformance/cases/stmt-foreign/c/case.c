/* The C source of the kit case stmt-foreign. */

/* Adds n to a total. A negative n jumps past the addition with a goto, which cir/1 cannot
   express; the calls never take that path. */
long count(long n) {
    long total = 0;
    if (n < 0) {
        goto done;
    }
    total += n;
done:
    return total;
}
