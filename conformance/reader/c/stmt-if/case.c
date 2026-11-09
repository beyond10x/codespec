/* The C source of the kit case stmt-if. */

/* 1 for a positive n, 2 for a negative one, 0 for zero. */
long sign(long n) {
    if (n > 0) {
        return 1;
    } else if (n < 0) {
        return 2;
    }
    return 0;
}
