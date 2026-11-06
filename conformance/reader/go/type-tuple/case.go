package type_tuple

func split(a int, b int) (int, int) {
	return b, a
}

// Flip binds both results of a call that returns two values: the call's type is a tuple.
func Flip(a int, b int) (int, int) {
	x, y := split(a, b)
	return x, y
}
