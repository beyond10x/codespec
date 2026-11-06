package assign_shr

// ShrBy shifts right in place.
func ShrBy(a int, b int) int {
	a >>= b
	return a
}
