package expr_zero_error

import "errors"

// case.cir.json is written by hand: the Go reader reads Go's zero literal 0 as a literal. The
// Java reader writes zero for the value a thrown sentinel exception returns beside its error.

var ErrNegative = errors.New("negative")

// Check returns n, or the zero int beside the sentinel error when n is negative.
func Check(n int) (int, error) {
	if n < 0 {
		return 0, ErrNegative
	}
	return n, nil
}
