package expr_field

// Pair holds two values.
type Pair struct {
	A int
	B int
}

// Right is the B of a pair.
func Right(a int, b int) int {
	p := Pair{A: a, B: b}
	return p.B
}
