// Package store keeps items.
package store

// Shelf holds items.
type Shelf struct {
	Items int
}

// Load gives the number of items.
func Load() int {
	return 3
}

// LoadCached gives the number of items, cached.
func LoadCached() int {
	return 4
}

// Fetch gives n items.
func Fetch(n int) int {
	return n
}

func spare() int {
	return 5
}

// Sum adds the items.
func Sum() int {
	return Load() + Fetch(1)
}

// size gives the items of the shelf.
// It is a method.
func (s Shelf) size() int {
	return s.Items
}

func (s Shelf) weight() int {
	return 2
}

func tidy() int {
	return 0
}
