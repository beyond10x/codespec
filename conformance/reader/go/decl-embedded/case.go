package decl_embedded

// Counter counts.
type Counter struct {
	N int
}

// Bump adds step to the count and gives the new count.
func (c *Counter) Bump(step int) int {
	c.N = c.N + step
	return c.N
}

// Tally embeds a Counter: its field N and its method Bump are promoted to Tally.
type Tally struct {
	Counter
	Step int
}

// Run bumps a tally by step k times through the promoted method and gives the promoted count.
func Run(step int, k int) int {
	t := &Tally{Step: step}
	for i := 0; i < k; i++ {
		t.Bump(t.Step)
	}
	return t.N
}
