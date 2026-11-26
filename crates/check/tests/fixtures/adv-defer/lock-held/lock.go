// Package lock defers a function literal that does not unlock: the early return still holds
// the lock.
package lock

import "sync"

// Counter counts keys.
type Counter struct {
	mu   sync.Mutex
	seen map[string]int
}

// Count counts k when it returns; it unlocks by hand on one path only.
func (c *Counter) Count(k string) int {
	c.mu.Lock()
	defer func() {
		c.seen[k]++
	}()
	if k == "" {
		return 0
	}
	c.mu.Unlock()
	return c.seen[k]
}
