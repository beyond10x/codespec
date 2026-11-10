/* The C source of the kit case sem-map-order, in runtime form: a loop over a map counts its index
   up to cs_map_len and reads each value through cs_map_val first. */

#include "codespec_rt.h"

/* codespec: func Squares(int) int */
int64_t Squares(int64_t n) {
    cs_map *m = cs_map_new(sizeof(int64_t), sizeof(int64_t), CS_KEY_I64);
    for (int64_t i = 0; i < n; i++) {
        *(int64_t *)cs_map_put(m, &i) = (int64_t)((uint64_t)i * (uint64_t)i);
    }
    int64_t s = 0;
    for (int64_t j = 0; j < cs_map_len(m); j++) {
        int64_t v = *(int64_t *)cs_map_val(m, j);
        s = (int64_t)((uint64_t)s + (uint64_t)v);
    }
    return s;
}
