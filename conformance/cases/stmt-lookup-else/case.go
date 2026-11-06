package stmt_lookup_else

import "errors"

var ErrMissing = errors.New("missing")

// Find looks a key up in a map of two entries.
func Find(k string) (int, error) {
	m := map[string]int{}
	m["a"] = 1
	m["b"] = 2
	v, ok := m[k]
	if !ok {
		return 0, ErrMissing
	}
	return v, nil
}
