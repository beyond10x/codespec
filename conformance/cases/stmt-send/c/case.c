/* The C source of the kit case stmt-send, in runtime form: a channel is a cs_chan, and a send
   copies a value of its element type in with cs_chan_send. */

#include "codespec_rt.h"

/* codespec: func Emit(chan int, int) */
void Emit(cs_chan *ch, int64_t n) {
    cs_chan_send(ch, &n);
    cs_chan_send(ch, &(int64_t){(int64_t)((uint64_t)n + (uint64_t)1)});
}
