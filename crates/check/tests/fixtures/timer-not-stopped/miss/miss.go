// Package miss arms three timers and stops all of them when it is torn down; a retry method
// stops one timer to arm it again.
package miss

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

// retry arms timer a again.
func (t *Tx) retry(d time.Duration) {
	t.a.Stop()
	t.a = time.AfterFunc(d, t.fire)
}

// Close stops the transaction's timers.
func (t *Tx) Close() {
	if t.a != nil {
		t.a.Stop()
	}
	if t.b != nil {
		t.b.Stop()
	}
	if t.m != nil {
		t.m.Stop()
	}
}
