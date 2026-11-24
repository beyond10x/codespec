// Package stock counts what is on the shelves.
package stock

// Load gives the number of items on the shelves.
func Load() int {
	return 3
}

// LoadCached gives the number of items as last counted.
func LoadCached() int {
	return 4
}

// Total adds two counts.
func Total() int {
	return Load() + Load()
}

// spare is called by nothing.
func spare() int {
	return 5
}

// Weight gives the weight of n items.
func Weight(n int) int {
	return n * 2
}
