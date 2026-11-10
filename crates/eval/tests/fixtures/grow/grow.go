// Package grow appends to a slice of five ints, whose capacity Go rounds up to six (the 48-byte
// size class), so two appends to it share one array.
package grow

// Alias is 10 * b[5] + c[5] for b and c appended to the same five-element slice.
func Alias() int {
	var a []int
	a = append(a, 1, 2, 3, 4, 5)
	b := append(a, 6)
	c := append(a, 7)
	return 10*b[5] + c[5]
}
