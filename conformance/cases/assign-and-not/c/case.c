/* The C source of the kit case assign-and-not. */

/* Clears the bits of b in a. */
long clearWith(long a, long b) {
    a &= ~b;
    return a;
}
