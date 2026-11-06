package type_unit_method

// case.cir.json is written by hand: the Go reader types a call of a method without results as an
// empty tuple. The Java and Rust readers type it as unit, the value of a call that gives none.

// Counter counts.
type Counter struct {
	N int
}

// Bump adds one.
func (c *Counter) Bump() {
	c.N++
}

// Count bumps a counter k times through a method that returns nothing.
func Count(k int) int {
	c := &Counter{}
	for i := 0; i < k; i++ {
		c.Bump()
	}
	return c.N
}
