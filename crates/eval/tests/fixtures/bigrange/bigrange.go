// Package bigrange ranges over a large integer, which only the step budget can end.
package bigrange

// Count counts to 1 << 60.
func Count() int {
	n := 0
	for range 1 << 60 {
		n++
	}
	return n
}
