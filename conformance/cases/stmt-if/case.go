package stmt_if

// Sign is 1 for a positive, 2 for a negative and 0 for zero.
func Sign(n int) int {
	if n > 0 {
		return 1
	} else if n < 0 {
		return 2
	}
	return 0
}
