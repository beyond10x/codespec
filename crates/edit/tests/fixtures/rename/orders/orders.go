// Package orders keeps orders.
package orders

// Cart holds lines.
type Cart struct {
	Lines int
}

// Load gives the number of lines.
func Load() int {
	return 3
}

// Reload loads again.
func Reload() int {
	return Load() + Load()
}

func (c Cart) count() int {
	return c.Lines
}

func (c Cart) total() int {
	return c.count() + Load()
}
