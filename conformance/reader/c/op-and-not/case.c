/* The C source of the kit case op-and-not. */

/* Clears the bits of b in a. */
long clear(long a, long b) {
    return a & ~b;
}
