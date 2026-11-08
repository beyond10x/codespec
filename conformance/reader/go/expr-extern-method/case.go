package expr_extern_method

import "strings"

// Count writes s into a builder of the standard library n times and gives its length: calls of
// methods of a type of another package.
func Count(s string, n int) int {
	var b strings.Builder
	for i := 0; i < n; i++ {
		b.WriteString(s)
	}
	return b.Len()
}
