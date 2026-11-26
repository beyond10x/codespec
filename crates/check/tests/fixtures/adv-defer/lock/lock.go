// Package lock releases its lock in a deferred function literal: every path returns with the
// lock released.
package lock

import "sync"

// Counter counts keys.
type Counter struct {
	mu   sync.Mutex
	seen map[string]int
}

// Count returns how often k was seen; the literal deferred after the lock unlocks it.
func (c *Counter) Count(k string) int {
	c.mu.Lock()
	defer func() {
		c.mu.Unlock()
	}()
	if k == "" {
		return 0
	}
	return c.seen[k]
}
