package expr_float

// Over tells whether a quarter of a float literal is more than half.
func Over(big bool) bool {
	var x float64
	x = 0.25
	if big {
		x = 1.5e300
	}
	return x/4 > 0.5
}
