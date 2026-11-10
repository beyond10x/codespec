/* The C source of the kit case expr-make, in runtime form: an empty map is cs_map_new with the
   sizes of its key and value types and the kind of its key. */

#include "codespec_rt.h"

/* codespec: func Fresh() int */
int64_t Fresh(void) {
    cs_map *m = cs_map_new(sizeof(cs_string), sizeof(int64_t), CS_KEY_STRING);
    return cs_map_len(m);
}
