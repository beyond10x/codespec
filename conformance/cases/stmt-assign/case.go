package stmt_assign

// Replace overwrites a local.
func Replace(a int, b int) int {
	x := a
	x = b
	return x
}
