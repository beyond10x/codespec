package expr_index

// At indexes a slice of two values.
func At(i int) int {
	xs := []int{}
	xs = append(xs, 10)
	xs = append(xs, 20)
	return xs[i]
}
