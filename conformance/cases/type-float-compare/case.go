package type_float_compare

// Third divides a 64-bit and a 32-bit float by three and tells whether the first quotient is
// more than 0.3 and the second less than 0.34: a variant of type-float whose result is a bool, not
// the quotients formatted as a string.
func Third(big bool) bool {
	var x float64
	var y float32
	x = 1
	y = 1
	if big {
		x = 1e21
		y = 1e21
	}
	return x/3 > 0.3 && y/3 < 0.34
}
