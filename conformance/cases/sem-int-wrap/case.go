package sem_int_wrap

// Add adds, wrapping on overflow.
func Add(a int, b int) int {
	return a + b
}

// Mul multiplies, wrapping on overflow.
func Mul(a int, b int) int {
	return a * b
}

// Div divides; MinInt64 / -1 wraps to MinInt64.
func Div(a int, b int) int {
	return a / b
}

// Rem is the remainder; MinInt64 % -1 is 0.
func Rem(a int, b int) int {
	return a % b
}

// Neg negates; -MinInt64 wraps to MinInt64.
func Neg(a int) int {
	return -a
}
