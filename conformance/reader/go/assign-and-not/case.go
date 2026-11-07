package assign_and_not

// ClearWith clears bits in place.
func ClearWith(a int, b int) int {
	a &^= b
	return a
}
