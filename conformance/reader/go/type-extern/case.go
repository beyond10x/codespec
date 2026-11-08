package type_extern

import "strings"

// Buffer holds text in a builder of the standard library: a field of a type of another package.
type Buffer struct {
	text *strings.Builder
	n    int
}

// Fill adds s to a new buffer n times and gives what the buffer holds and how many times.
func Fill(s string, n int) string {
	var text strings.Builder
	b := Buffer{text: &text}
	for i := 0; i < n; i++ {
		b.text.WriteString(s)
		b.n++
	}
	return b.text.String() + strings.Repeat("!", b.n)
}
