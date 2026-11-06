package assign_and

// AndWith ands in place.
func AndWith(a int, b int) int {
	a &= b
	return a
}
