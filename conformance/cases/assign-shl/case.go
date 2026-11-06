package assign_shl

// ShlBy shifts left in place.
func ShlBy(a int, b int) int {
	a <<= b
	return a
}
