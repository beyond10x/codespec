package fact_comma_ok

// Positive gives n and true for a positive n.
func Positive(n int) (int, bool) {
	if n > 0 {
		return n, true
	}
	return 0, false
}
