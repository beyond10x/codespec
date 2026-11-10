/* The C source of the kit case type-array-int. */

/* Keeps three integers in an array, each set from the ones before it; gives the one at i. */
long pick(long i) {
    long a[3] = {0};
    a[0] = 10;
    a[1] = a[0] * 2;
    a[2] = a[1] + a[0];
    return a[i];
}
