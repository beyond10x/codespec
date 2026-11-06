package unop_deref

// Through writes a local through a pointer to it.
func Through(n int) int {
	x := 0
	p := &x
	*p = n
	return x
}
