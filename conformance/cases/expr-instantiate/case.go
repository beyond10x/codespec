package expr_instantiate

// Box holds one value of any type.
type Box[T any] struct {
	v T
}

func get[T any](b Box[T]) T {
	return b.v
}

// Wrap puts n in a box of int and takes it out again.
func Wrap(n int) int {
	b := Box[int]{v: n}
	return get[int](b)
}
