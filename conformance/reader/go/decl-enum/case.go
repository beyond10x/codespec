package decl_enum

// Color is one of two colours.
type Color int

const (
	Red   Color = 0
	Green Color = 1
)

// Pick is Red for zero and Green otherwise.
func Pick(n int) Color {
	if n == 0 {
		return Red
	}
	return Green
}
