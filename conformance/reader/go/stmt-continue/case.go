package stmt_continue

// SkipSum sums the values up to n-1 except skip.
func SkipSum(n int, skip int) int {
	xs := []int{}
	for i := 0; i < n; i++ {
		xs = append(xs, i)
	}
	s := 0
	for _, x := range xs {
		if x == skip {
			continue
		}
		s += x
	}
	return s
}
