/* The C source of the kit case decl-struct. */

/* A range. */
struct Span {
    long from;
    long to;
};

/* The width of the span from..to. */
long width(long from, long to) {
    struct Span s = {.from = from, .to = to};
    return s.to - s.from;
}
