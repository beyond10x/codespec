package expr_lambda

// Scale multiplies n by k twice through a function literal that captures k, and adds how often
// the literal ran, counted in a captured local it writes.
func Scale(n int, k int) int {
	calls := 0
	mul := func(x int) int {
		calls++
		return x * k
	}
	r := mul(mul(n))
	return r + calls
}
