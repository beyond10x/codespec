package expr_spread_local

// sum adds its arguments.
func sum(xs ...int) int {
	t := 0
	for _, x := range xs {
		t = t + x
	}
	return t
}

// Twice builds a slice of a, b, a and b and passes it to sum spread: 2*(a+b).
func Twice(a int, b int) int {
	xs := append([]int{}, a, b, a, b)
	return sum(xs...)
}
