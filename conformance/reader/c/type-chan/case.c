/* The C source of the kit case type-chan, in runtime form: a channel parameter is a cs_chan, its
   element type the codespec: func line's. */

#include "codespec_rt.h"

/* codespec: func Pipe(chan string, int) */
void Pipe(cs_chan *ch, int64_t n) {
    for (int64_t i = 0; i < n; i++) {
        cs_chan_send(ch, &(cs_string){"v", 1});
    }
}
