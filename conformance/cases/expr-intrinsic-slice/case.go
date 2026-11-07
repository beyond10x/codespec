package expr_intrinsic_slice

// Size is the length of a slice holding n words.
func Size(n int) int {
	words := []string{}
	for i := 0; i < n; i++ {
		words = append(words, "w")
	}
	return len(words)
}
