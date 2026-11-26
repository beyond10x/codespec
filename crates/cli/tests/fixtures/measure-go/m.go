package m

// Add is read.
func Add(a, b int) int { return a + b }

// Mag has a complex128 signature: the reader records it as opaque.
func Mag(c complex128) float64 { return real(c) }

// Run holds a local closure whose body starts a goroutine: the closure's body is foreign.
func Run(f func()) int {
	g := func() int {
		go f()
		return 1
	}
	return g()
}
