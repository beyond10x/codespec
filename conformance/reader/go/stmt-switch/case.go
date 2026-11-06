package stmt_switch

// Name names a count.
func Name(n int) string {
	switch n {
	case 1:
		return "one"
	case 2, 3:
		return "few"
	}
	return "many"
}
