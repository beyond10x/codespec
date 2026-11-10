/* The C source of the kit case decl-var, in runtime form: a file-scope cs_string with a literal
   initializer, and a counter at its zero value. */

#include "codespec_rt.h"

cs_string greeting = {"hello, ", 7};

int64_t hits;

/* codespec: func Greet(string) string */
cs_string Greet(cs_string name) {
    hits = (int64_t)((uint64_t)hits + (uint64_t)1);
    return cs_string_cat(greeting, name);
}

/* codespec: func Calls() int */
int64_t Calls(void) {
    return hits;
}
