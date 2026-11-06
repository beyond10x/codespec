package decl_newtype

// Celsius is a temperature.
type Celsius int

// Warm is one degree warmer.
func Warm(n int) Celsius {
	c := Celsius(n)
	return c + 1
}
