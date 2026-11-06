package expr_make

// Fresh counts the entries of an empty map.
func Fresh() int {
	m := map[string]int{}
	return len(m)
}
