package type_error

import "errors"

var ErrBad = errors.New("bad")

// Fail fails when told to.
func Fail(bad bool) error {
	if bad {
		return ErrBad
	}
	return nil
}
