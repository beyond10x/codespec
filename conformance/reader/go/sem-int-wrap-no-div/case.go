package sem_int_wrap_no_div

// Add adds, wrapping on overflow.
func Add(a int, b int) int {
	return a + b
}

// Sub subtracts, wrapping on overflow.
func Sub(a int, b int) int {
	return a - b
}

// Mul multiplies, wrapping on overflow.
func Mul(a int, b int) int {
	return a * b
}

// Neg negates; -MinInt64 wraps to MinInt64.
func Neg(a int) int {
	return -a
}
