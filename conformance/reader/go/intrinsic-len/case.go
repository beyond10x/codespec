package intrinsic_len

// Count is the length of a slice of n values.
func Count(n int) int {
	xs := []string{}
	for i := 0; i < n; i++ {
		xs = append(xs, "x")
	}
	return len(xs)
}
