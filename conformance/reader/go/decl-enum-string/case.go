package decl_enum_string

// Color is one of two colours, each a string holding its own name, as a Java enum is read.
type Color string

const (
	Red   Color = "Red"
	Green Color = "Green"
)

// Pick is Red for zero and Green otherwise.
func Pick(n int) Color {
	if n == 0 {
		return Red
	}
	return Green
}
