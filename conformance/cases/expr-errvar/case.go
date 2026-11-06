package expr_errvar

import "errors"

var ErrEmpty = errors.New("empty")

// Check refuses the empty string.
func Check(s string) error {
	if s == "" {
		return ErrEmpty
	}
	return nil
}
