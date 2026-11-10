package type_array_int

// Pick keeps three integers in a fixed-size array, each set from the ones before it, and gives
// the one at i: a variant of type-array-float whose result is an integer, not floats formatted
// as a string.
func Pick(i int) int {
	var a [3]int
	a[0] = 10
	a[1] = a[0] * 2
	a[2] = a[1] + a[0]
	return a[i]
}
