package fact_unordered_iteration

// Keys counts the keys of a map with n entries.
func Keys(n int) int {
	m := map[int]bool{}
	for i := 0; i < n; i++ {
		m[i] = true
	}
	c := 0
	for range m {
		c++
	}
	return c
}
