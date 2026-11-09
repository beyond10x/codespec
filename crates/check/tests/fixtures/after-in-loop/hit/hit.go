// Package hit retransmits on a timer it resets and gives up with a time.After made inside the
// loop: every turn starts the give-up wait over, so it never fires while the timer runs.
package hit

import (
	"context"
	"time"
)

// Retransmit sends every interval until ctx ends, giving up after a minute.
func Retransmit(send func(), ctx context.Context) {
	t := time.NewTimer(time.Second)
	defer t.Stop()
	for {
		select {
		case <-t.C:
			send()
			t.Reset(2 * time.Second)
		case <-time.After(time.Minute):
			return
		case <-ctx.Done():
			return
		}
	}
}
