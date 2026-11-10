/* The C source of the kit case decl-newtype. */

/* A temperature. */
typedef long Celsius;

/* One degree warmer. */
Celsius warm(long n) {
    Celsius c = (Celsius)n;
    return c + 1;
}
