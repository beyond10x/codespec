// Package foreign starts a goroutine on one branch only: the Go reader keeps it as a foreign
// statement, which a run stops at when it executes it and passes when it does not.
package foreign

// Count returns n, starting a goroutine first when n is positive.
func Count(n int) int {
	total := 0
	if n > 0 {
		go done()
	}
	total += n
	return total
}

func done() {}
