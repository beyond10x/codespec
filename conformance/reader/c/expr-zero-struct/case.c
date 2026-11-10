/* The C source of the kit case expr-zero-struct. */

/* A position. */
struct Point {
    long x;
    long y;
};

/* The sum of the coordinates of the point (n, n), or of the zero point when n is not positive. */
long sum(long n) {
    struct Point p = {n, n};
    if (n <= 0) {
        p = (struct Point){0};
    }
    return p.x + p.y;
}
