package type_ptr

// Bump increments a value through a pointer.
func Bump(n int) int {
	p := &n
	*p = *p + 1
	return n
}
