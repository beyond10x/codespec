package expr_call

func twice(n int) int {
	return n + n
}

// Quad calls a module function twice.
func Quad(n int) int {
	return twice(twice(n))
}
