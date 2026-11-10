package expr_nil_ptr

// Found points p at n when n is positive, else leaves it nil, and tells whether p is set: a
// variant of expr-nil whose nil is a pointer, not a slice or an error.
func Found(n int) bool {
	var p *int
	if n > 0 {
		p = &n
	}
	return p != nil
}
