/* The C source of the kit case type-map, in runtime form: a map is a cs_map; an entry is
   assigned through cs_map_put and read through cs_map_get, each cast to a pointer to the value
   type, the key the address of a variable of the key type. */

#include "codespec_rt.h"

/* codespec: func Stored(string) int */
int64_t Stored(cs_string s) {
    cs_map *m = cs_map_new(sizeof(cs_string), sizeof(int64_t), CS_KEY_STRING);
    *(int64_t *)cs_map_put(m, &s) = s.n;
    return *(int64_t *)cs_map_get(m, &s, sizeof(int64_t));
}
