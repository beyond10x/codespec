package fact_fallible

import "errors"

var ErrNegative = errors.New("negative")

// Checked refuses a negative.
func Checked(n int) (int, error) {
	if n < 0 {
		return 0, ErrNegative
	}
	return n, nil
}
