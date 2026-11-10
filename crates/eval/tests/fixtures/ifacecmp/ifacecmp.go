// Package ifacecmp compares interface values with concrete ones and keys a map by them, where
// Go converts the concrete operand to the interface type first.
package ifacecmp

// Named is an int of its own type.
type Named int

// IsOne reports whether n held in an `any` equals 1.
func IsOne(n int) bool {
	var a any
	a = n
	return a == 1
}

// Which names n through a switch on it held in an `any`.
func Which(n int) string {
	var a any
	a = n
	switch a {
	case 1:
		return "one"
	}
	return "other"
}

// Keys counts the keys of a map keyed by `any` after storing 1 and Named(1).
func Keys() int {
	m := map[any]int{}
	var n Named
	n = 1
	m[1] = 1
	m[n] = 2
	return len(m)
}

// Held reports whether a Named(1) appended to a []any equals the int 1, or differs from the
// Named(1): neither, as an append converts n to any keeping its type.
func Held() bool {
	var n Named
	n = 1
	xs := []any{}
	xs = append(xs, n)
	return xs[0] == 1 || xs[0] != n
}
