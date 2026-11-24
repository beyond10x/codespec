// Package pair picks one of two numbers.
package pair

// First gives the first of two.
func First(a, b int) int {
	return pick(a, b, true)
}

func pick(a, b int, first bool) int {
	if first {
		return a
	}
	return b
}
