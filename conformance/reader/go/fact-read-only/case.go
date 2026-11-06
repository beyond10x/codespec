package fact_read_only

// Cell holds a value.
type Cell struct {
	V int
}

// Get reads the value.
func (c *Cell) Get() int {
	return c.V
}

// Read builds a cell and reads it.
func Read(v int) int {
	c := &Cell{V: v}
	return c.Get()
}
