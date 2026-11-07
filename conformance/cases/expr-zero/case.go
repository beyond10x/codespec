package expr_zero

import "fmt"

// case.cir.json is written by hand: the Go reader reads Go's zero literals 0 and "" as literals.
// The Java reader writes zero for the value returned beside an error or a false comma-ok, and
// the Rust reader for the zero of a type it has no literal for.

// Check returns n, or the zero int beside an error when n is negative.
func Check(n int) (int, error) {
	if n < 0 {
		return 0, fmt.Errorf("negative: %d", n)
	}
	return n, nil
}

// Pick returns s, or the zero string when keep is false.
func Pick(keep bool, s string) string {
	if keep {
		return s
	}
	return ""
}
