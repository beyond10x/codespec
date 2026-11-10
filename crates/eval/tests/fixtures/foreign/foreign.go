// Package foreign defers a call on one branch only: the Go reader keeps the defer as a foreign
// statement, which a run stops at when it executes it and passes when it does not.
package foreign

// Count returns n, deferring a call first when n is positive.
func Count(n int) int {
	total := 0
	if n > 0 {
		defer done()
	}
	total += n
	return total
}

func done() {}
