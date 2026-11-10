package rangeforms

// ParenBlank assigns only a blank, written in parentheses: Go gives len(xs).
func ParenBlank(xs []int) int {
	n := 0
	for (_) = range xs {
		n++
	}
	return n
}

// Labelled assigns i, declared before the loop, under a label only its own break names.
func Labelled(xs []int) int {
	i := -1
L:
	for i = range xs {
		if i == 1 {
			break L
		}
	}
	return i
}
