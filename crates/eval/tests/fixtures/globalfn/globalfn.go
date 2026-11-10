// Package globalfn divides in a function literal held by a package variable.
package globalfn

var div = func(x int) int { return 10 / x }

// Div is 10 / x through the literal.
func Div(x int) int {
	return div(x)
}
