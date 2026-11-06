package intrinsic_append

// Build collects 0 up to n-1.
func Build(n int) []int {
	xs := []int{}
	for i := 0; i < n; i++ {
		xs = append(xs, i)
	}
	return xs
}
