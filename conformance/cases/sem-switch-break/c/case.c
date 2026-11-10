/* The C source of the kit case sem-switch-break, in runtime form: a switch is a block that holds
   the tag in a temporary cs_s<n> and tests it in a chain of if and else if, so a break inside a
   case leaves the loop around it, as a break in a cir/1 switch does. */

#include "codespec_rt.h"

/* codespec: func Until(int, int) int */
int64_t Until(int64_t n, int64_t stop) {
    int64_t i = 0;
    while (i < n) {
        {
            int64_t cs_s1 = i;
            if (cs_s1 == stop) {
                break;
            }
        }
        i++;
    }
    return i;
}
