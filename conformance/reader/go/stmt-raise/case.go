package stmt_raise

// case.cir.json is written by hand: Go has no statement that leaves a function, and every caller
// up to the framework that started it, with a value of another package's type. The Java reader
// reads `throw new IllegalArgumentException("negative")` in a method an entry names as one. The
// calls never reach it, since the kit's driver is no framework to hand the value to: the case
// checks that a writer writes the function or refuses it by name, and what it returns otherwise.

// Check returns n; for a negative n it panics, where the module raises.
func Check(n int) int {
	if n < 0 {
		panic("negative")
	}
	return n
}
