package stmt_for

// Below sums 0 up to n-1.
func Below(n int) int {
	s := 0
	for i := 0; i < n; i++ {
		s += i
	}
	return s
}
