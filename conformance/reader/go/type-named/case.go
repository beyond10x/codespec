package type_named

// Meters is a length.
type Meters int

// Measure converts to Meters.
func Measure(n int) Meters {
	m := Meters(n)
	return m
}
