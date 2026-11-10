/* The C source of the kit case type-float-compare. */

#include <stdbool.h>

/* Divides a double and a float by three; tells whether the first quotient is more than 0.3 and
   the second less than 0.34. */
bool third(bool big) {
    double x;
    float y;
    x = 1.0;
    y = 1.0f;
    if (big) {
        x = 1e21;
        y = 1e21f;
    }
    return x / 3.0 > 0.3 && y / 3.0f < 0.34f;
}
