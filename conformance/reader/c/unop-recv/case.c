/* The C source of the kit case unop-recv, in runtime form: a receive is cs_chan_recv, cast to a
   pointer to the element type. */

#include "codespec_rt.h"

/* codespec: func Echo(chan int, int) int */
int64_t Echo(cs_chan *ch, int64_t n) {
    cs_chan_send(ch, &n);
    return *(int64_t *)cs_chan_recv(ch);
}
