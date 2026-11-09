/* The C source of the kit case decl-enum. */

/* One of two colours. */
typedef enum { Red = 0, Green = 1 } Color;

/* Red for zero and Green otherwise. */
Color pick(long n) {
    if (n == 0) {
        return Red;
    }
    return Green;
}
