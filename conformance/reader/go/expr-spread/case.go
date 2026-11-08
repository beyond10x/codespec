package expr_spread

import "path"

// sum adds its arguments.
func sum(xs ...int) int {
	t := 0
	for _, x := range xs {
		t = t + x
	}
	return t
}

// Twice builds a slice of a and b, appends it to itself spread and passes the result to sum
// spread: 2*(a+b).
func Twice(a int, b int) int {
	xs := append([]int{}, a, b)
	ys := append(xs, xs...)
	return sum(ys...)
}

// Joined passes a slice to a variadic standard library function spread.
func Joined(a string, b string) string {
	parts := append([]string{}, a, b)
	return path.Join(parts...)
}
