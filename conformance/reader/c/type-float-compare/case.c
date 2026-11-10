/* The C source of the kit case type-float-compare. */

#include <stdbool.h>

/* Tells whether a tenth plus two tenths is three tenths: in doubles it is not, in floats the sum
   rounds to three tenths. */
bool exact(bool wide) {
    if (wide) {
        double x;
        double y;
        x = 0.1;
        y = 0.2;
        return x + y == 0.3;
    }
    float a;
    float b;
    a = 0.1f;
    b = 0.2f;
    return a + b == 0.3f;
}

/* Tells whether a third of y, times three, is y again, in floats. */
bool third(bool big) {
    float y;
    y = 1.0f;
    if (big) {
        y = 1e21f;
    }
    return y / 3.0f * 3.0f == y;
}
