package assign_rem

// RemBy takes the remainder in place.
func RemBy(a int, b int) int {
	a %= b
	return a
}
