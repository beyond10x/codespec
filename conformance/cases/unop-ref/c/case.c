/* The C source of the kit case unop-ref. */

/* Holds a value. */
struct Box {
    long v;
};

/* Reads a value through a pointer to a fresh box. */
long boxed(long v) {
    struct Box *b = &(struct Box){v};
    return b->v;
}
