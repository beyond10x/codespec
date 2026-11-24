package store

// Spread gives one item of the named kind.
func Spread(name string) Item {
	return Item{
		Name:   name,
		Count:  1,
	}
}
