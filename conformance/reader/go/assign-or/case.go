package assign_or

// OrWith ors in place.
func OrWith(a int, b int) int {
	a |= b
	return a
}
