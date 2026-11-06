package decl_struct

// Span is a range.
type Span struct {
	From int
	To   int
}

// Width is the width of a span.
func Width(from int, to int) int {
	s := Span{From: from, To: to}
	return s.To - s.From
}
