/* The C source of the kit case expr-struct. */

/* A point. */
struct Point {
    long x;
    long y;
};

/* Builds a point and reads x back. */
long make(long x) {
    struct Point p = {.x = x, .y = 0};
    return p.x;
}
