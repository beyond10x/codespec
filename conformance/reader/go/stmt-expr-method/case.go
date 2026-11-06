package stmt_expr_method

// Counter counts.
type Counter struct {
	N int
}

// Bump adds one.
func (c *Counter) Bump() {
	c.N++
}

// Count bumps a counter k times, each call a statement of its own.
func Count(k int) int {
	c := &Counter{}
	for i := 0; i < k; i++ {
		c.Bump()
	}
	return c.N
}
