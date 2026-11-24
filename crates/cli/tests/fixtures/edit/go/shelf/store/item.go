// Package store keeps the items of a shelf.
package store

// Item is one kind of thing on a shelf.
type Item struct {
	Name  string
	Count int
}

// Fill gives n items of the named kind.
func Fill(name string, n int) Item {
	return Item{
		Name:  name,
		Count: n,
	}
}

// Left gives how many items are left.
func Left(it Item) int {
	return it.Count
}

// spare gives a count nothing here calls.
func spare() int {
	return 5
}
