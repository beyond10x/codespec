// Package miss makes its give-up wait once, before the loop; a retry loop waits with
// time.After on every turn on purpose, and resets no timer.
package miss

import (
	"context"
	"time"
)

// Retransmit sends every interval until ctx ends, giving up after a minute.
func Retransmit(send func(), ctx context.Context) {
	t := time.NewTimer(time.Second)
	defer t.Stop()
	giveUp := time.After(time.Minute)
	for {
		select {
		case <-t.C:
			send()
			t.Reset(2 * time.Second)
		case <-giveUp:
			return
		case <-ctx.Done():
			return
		}
	}
}

// Retry calls try until it succeeds or ctx ends, waiting a second between tries.
func Retry(try func() bool, ctx context.Context) {
	for !try() {
		select {
		case <-time.After(time.Second):
		case <-ctx.Done():
			return
		}
	}
}
