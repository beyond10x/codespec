// Package ifacemut holds conversions to an interface the pass-1 fixture does not reach: the
// concrete operand on the left, a concrete switch tag, a map read and a comma-ok lookup with a
// concrete key, a package variable's initializer, and an append of bools.
package ifacemut

// Left compares 1 with an any holding 1, the concrete operand on the left.
func Left() bool {
	var a any
	a = 1
	return 1 == a
}

// Tag switches on 1 against a case holding an any that holds 1.
func Tag() string {
	var a any
	a = 1
	switch 1 {
	case a:
		return "one"
	}
	return "other"
}

// Read reads the entry a map keyed by any holds for the int 1.
func Read() int {
	m := map[any]int{}
	m[1] = 5
	return m[1]
}

// Lookup looks the int 1 up in a map keyed by any.
func Lookup() int {
	m := map[any]int{}
	m[1] = 5
	v, ok := m[1]
	if !ok {
		return -1
	}
	return v
}

var g any = 1

// Global compares a package variable of type any initialized to 1 with 1.
func Global() bool { return g == 1 }

// Bools appends twice to a slice holding one bool, whose capacity Go rounds up to 8.
func Bools() bool {
	var a []bool
	a = append(a, true)
	b := append(a, false)
	c := append(a, true)
	return b[1] && c[1]
}

// Appended reports whether the int 1 appended to a []any equals 1.
func Appended() bool {
	xs := []any{}
	xs = append(xs, 1)
	return xs[0] == 1
}
