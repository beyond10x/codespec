// Package dropped parses numbers with strconv.Atoi, each function doing its own thing with the
// error: Count binds it to _, Parse returns it, OrMinus tests it.
package dropped

import "strconv"

// Count gives the number s spells, 0 when it spells none: the error is dropped.
func Count(s string) int {
	n, _ := strconv.Atoi(s)
	return n
}

// Parse gives the number s spells and the parse's error to its caller.
func Parse(s string) (int, error) {
	return strconv.Atoi(s)
}

// OrMinus gives the number s spells, -1 when it spells none.
func OrMinus(s string) int {
	n, err := strconv.Atoi(s)
	if err != nil {
		return -1
	}
	return n
}
