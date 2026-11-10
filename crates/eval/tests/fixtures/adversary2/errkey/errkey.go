// Package errkey holds a value of an error type once as an error and once as itself, where Go
// compares the two as one dynamic type and value.
package errkey

// E is an error type.
type E struct{ c int }

// Error names an E.
func (e E) Error() string { return "e" }

func mk() error { return E{c: 1} }

// ErrKeys keys a map[any]int by the error E{c: 1} and by E{c: 1}.
func ErrKeys() int {
	m := map[any]int{}
	err := mk()
	m[err] = 1
	m[E{c: 1}] = 2
	return len(m)
}

// ErrLit compares an any holding E{c: 1} with the literal E{c: 1}.
func ErrLit() bool {
	var x any
	x = E{c: 1}
	return x == E{c: 1}
}
