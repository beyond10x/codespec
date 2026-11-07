package sem_error_propagate

import "errors"

var ErrNegative = errors.New("negative")

// Checked refuses a negative.
func Checked(n int) (int, error) {
	if n < 0 {
		return 0, ErrNegative
	}
	return n, nil
}

// Valid refuses a negative.
func Valid(n int) error {
	if n < 0 {
		return ErrNegative
	}
	return nil
}

// Through returns Checked's error unchanged, or its value.
func Through(n int) (int, error) {
	v, err := Checked(n)
	if err != nil {
		return 0, err
	}
	return v, nil
}

// Guarded returns Valid's error unchanged, or n.
func Guarded(n int) (int, error) {
	err := Valid(n)
	if err != nil {
		return 0, err
	}
	return n, nil
}
