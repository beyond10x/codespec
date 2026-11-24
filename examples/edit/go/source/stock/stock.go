// Package stock counts the goods of a depot.
package stock

// Item is one kind of good on the shelves.
type Item struct {
	Name  string
	Count int
}

// Load gives the number of goods on the shelves.
func Load() int {
	return 3
}

// LoadCached gives the number of goods as last counted.
func LoadCached() int {
	return 4
}

// Total adds two counts.
func Total() int {
	return Load() + Load()
}

// Fill gives n goods of the named kind.
func Fill(name string, n int) Item {
	return Item{
		Name:  name,
		Count: n,
	}
}

// Left gives how many goods of an item are left.
func Left(it Item) int {
	return it.Count
}

// weigh gives the weight of n goods.
func weigh(n int) int {
	return n * 2
}

// Heavy gives the weight of the goods on the shelves.
func Heavy() int {
	heft := Load()
	return weigh(heft)
}

// spare gives a count nothing calls.
func spare() int {
	return 5
}
