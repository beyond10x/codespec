/* The C source of the kit case stmt-lookup-else, in runtime form: a lookup that may miss is
   cs_map_find, a null result the miss, and the value read through the pointer after it. */

#include "codespec_rt.h"

static struct cs_error_s ErrMissing_s = {{"missing", 7}, NULL, NULL};
cs_error const ErrMissing = &ErrMissing_s;

typedef struct {
    int64_t r0;
    cs_error r1;
} Find_result;

/* codespec: func Find(string) (int, error) */
Find_result Find(cs_string k) {
    cs_map *m = cs_map_new(sizeof(cs_string), sizeof(int64_t), CS_KEY_STRING);
    *(int64_t *)cs_map_put(m, &(cs_string){"a", 1}) = 1;
    *(int64_t *)cs_map_put(m, &(cs_string){"b", 1}) = 2;
    int64_t *cs_found1 = cs_map_find(m, &k);
    if (cs_found1 == NULL) {
        return (Find_result){0, ErrMissing};
    }
    int64_t v = *cs_found1;
    return (Find_result){v, NULL};
}
