/* The C source of the kit case intrinsic-sorted-keys, in runtime form: a map's keys in ascending
   order are cs_map_sorted_keys, a slice of the key type. */

#include "codespec_rt.h"

/* codespec: func Join(string, string, string) string */
cs_string Join(cs_string a, cs_string b, cs_string c) {
    cs_map *m = cs_map_new(sizeof(cs_string), sizeof(int64_t), CS_KEY_STRING);
    *(int64_t *)cs_map_put(m, &a) = 1;
    *(int64_t *)cs_map_put(m, &b) = 2;
    *(int64_t *)cs_map_put(m, &c) = 3;
    cs_slice keys = cs_map_sorted_keys(m);
    cs_string s = (cs_string){"", 0};
    for (int64_t i = 0; i < keys.len; i++) {
        cs_string k = *(cs_string *)cs_slice_at(keys, i, sizeof(cs_string));
        s = cs_string_cat(s, k);
    }
    return s;
}

/* codespec: func Ints(int, int, int) []int */
cs_slice Ints(int64_t a, int64_t b, int64_t c) {
    cs_map *m = cs_map_new(sizeof(int64_t), sizeof(int64_t), CS_KEY_I64);
    *(int64_t *)cs_map_put(m, &a) = a;
    *(int64_t *)cs_map_put(m, &b) = b;
    *(int64_t *)cs_map_put(m, &c) = c;
    return cs_map_sorted_keys(m);
}
