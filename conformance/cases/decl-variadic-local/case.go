package decl_variadic_local

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
