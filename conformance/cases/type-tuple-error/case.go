package type_tuple_error

import "errors"

var ErrNegative = errors.New("negative")

func doubled(n int) (int, error) {
	if n < 0 {
		return 0, ErrNegative
	}
	return n * 2, nil
}

// Quadruple doubles n twice, binding both results of each call: a call that returns a value and
// an error is of a tuple type.
func Quadruple(n int) (int, error) {
	d, err := doubled(n)
	if err != nil {
		return 0, err
	}
	q, err2 := doubled(d)
	if err2 != nil {
		return 0, err2
	}
	return q, nil
}
