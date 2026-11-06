package unop_ref

// Box holds a value.
type Box struct {
	V int
}

// Boxed reads a value through a reference to a fresh box.
func Boxed(v int) int {
	b := &Box{V: v}
	return b.V
}
