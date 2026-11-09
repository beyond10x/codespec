// Package miss releases its lock on every path: by hand before an early return, deferred, or
// under the same condition it took it; a function named for locking holds it for its caller.
package miss

import "sync"

// Counter counts keys.
type Counter struct {
	mu   sync.Mutex
	seen map[string]int
}

// Count returns how often k was seen.
func (c *Counter) Count(k string) int {
	c.mu.Lock()
	if k == "" {
		c.mu.Unlock()
		return 0
	}
	n := c.seen[k]
	c.mu.Unlock()
	return n
}

// CountDeferred returns how often k was seen.
func (c *Counter) CountDeferred(k string) int {
	c.mu.Lock()
	defer c.mu.Unlock()
	if k == "" {
		return 0
	}
	return c.seen[k]
}

// Forget drops k, taking the lock only when the counter is shared.
func (c *Counter) Forget(k string, shared bool) {
	if shared {
		c.mu.Lock()
	}
	delete(c.seen, k)
	if shared {
		c.mu.Unlock()
	}
}

// lock takes the counter's lock for its caller, which releases it.
func (c *Counter) lock() {
	c.mu.Lock()
}

// each runs f on every key with the lock released around the call; its caller holds the lock.
func (c *Counter) each(f func(string)) {
	for k := range c.seen {
		c.mu.Unlock()
		f(k)
		c.mu.Lock()
	}
}

// Reset clears the counter; release unlocks.
func (c *Counter) Reset() {
	c.mu.Lock()
	c.seen = map[string]int{}
	c.release()
}

func (c *Counter) release() {
	c.mu.Unlock()
}
