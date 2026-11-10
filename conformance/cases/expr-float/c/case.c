/* The C source of the kit case expr-float. */

#include <stdbool.h>

/* Tells whether a quarter of a float literal is more than half. */
bool over(bool big) {
    double x;
    x = 0.25;
    if (big) {
        x = 1.5e300;
    }
    return x / 4.0 > 0.5;
}
