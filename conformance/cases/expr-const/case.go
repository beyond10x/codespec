package expr_const

// Limit is the largest value Clamp returns.
const Limit = 10

// Clamp caps n at Limit.
func Clamp(n int) int {
	if n > Limit {
		return Limit
	}
	return n
}
