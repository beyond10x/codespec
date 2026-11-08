package decl_variadic

import "path"

// sum adds its arguments; called with none it is 0.
func sum(xs ...int) int {
	t := 0
	for _, x := range xs {
		t = t + x
	}
	return t
}

// Sums calls sum with no, one and three arguments.
func Sums(a int, b int) int {
	return sum() + sum(a) + sum(a, b, 3)
}

// Joined joins three elements into a slash-separated path through a variadic standard library
// function.
func Joined(a string, b string) string {
	return path.Join(a, b, "c")
}
