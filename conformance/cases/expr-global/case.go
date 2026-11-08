package expr_global

var total int

// Add adds n to the running total and returns the new total.
func Add(n int) int {
	total = total + n
	return total
}
