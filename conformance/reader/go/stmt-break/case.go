package stmt_break

// Root is the smallest i with i*i > n.
func Root(n int) int {
	i := 0
	for {
		if i*i > n {
			break
		}
		i++
	}
	return i
}
