package op_and_not

// Clear is a and not b.
func Clear(a int, b int) int {
	return a &^ b
}
