// Package hit arms three timers and stops two of them when it is torn down.
package hit

import "time"

// Tx is a transaction with its timers.
type Tx struct {
	a *time.Timer
	b *time.Timer
	m *time.Timer
}

func (t *Tx) start(d time.Duration) {
	t.a = time.AfterFunc(d, t.fire)
	t.b = time.AfterFunc(d, t.fire)
}

func (t *Tx) accept(d time.Duration) {
	t.m = time.AfterFunc(d, t.fire)
}

func (t *Tx) fire() {}

// Close stops the transaction's timers.
func (t *Tx) Close() {
	if t.a != nil {
		t.a.Stop()
	}
	if t.b != nil {
		t.b.Stop()
	}
}
