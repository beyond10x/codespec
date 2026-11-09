// Package wrapped compares with != an error whose function wraps the sentinel with %w.
package wrapped

import (
	"errors"
	"fmt"
)

// ErrMissing is the error of a lookup that finds nothing.
var ErrMissing = errors.New("missing")

func find(m map[string]int, k string) (int, error) {
	v, ok := m[k]
	if !ok {
		return 0, fmt.Errorf("find %s: %w", k, ErrMissing)
	}
	return v, nil
}

// Has reports whether m holds k.
func Has(m map[string]int, k string) bool {
	_, err := find(m, k)
	return err != ErrMissing
}
