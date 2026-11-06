package stmt_for_range

// Total sums the first n naturals collected in a slice.
func Total(n int) int {
	xs := []int{}
	for i := 0; i < n; i++ {
		xs = append(xs, i)
	}
	s := 0
	for _, x := range xs {
		s += x
	}
	return s
}
