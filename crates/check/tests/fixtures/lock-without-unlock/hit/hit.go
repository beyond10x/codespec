// Package hit returns early while it holds its lock.
package hit

import "sync"

// Counter counts keys.
type Counter struct {
	mu   sync.Mutex
	seen map[string]int
}

// Count returns how often k was seen; an empty key returns with the lock held.
func (c *Counter) Count(k string) int {
	c.mu.Lock()
	if k == "" {
		return 0
	}
	n := c.seen[k]
	c.mu.Unlock()
	return n
}
