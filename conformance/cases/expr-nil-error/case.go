package expr_nil_error

import "errors"

var ErrNegative = errors.New("negative")

// Check returns n beside a nil error, or the sentinel error when n is negative.
func Check(n int) (int, error) {
	if n < 0 {
		return 0, ErrNegative
	}
	return n, nil
}
