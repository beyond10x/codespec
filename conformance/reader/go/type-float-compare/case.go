package type_float_compare

// Exact tells whether a tenth plus two tenths is three tenths: in 64-bit floats it is not, in
// 32-bit floats the sum rounds to three tenths.
func Exact(wide bool) bool {
	if wide {
		var x float64
		var y float64
		x = 0.1
		y = 0.2
		return x+y == 0.3
	}
	var a float32
	var b float32
	a = 0.1
	b = 0.2
	return a+b == 0.3
}

// Third tells whether a third of n, times three, is n again, in 32-bit floats.
func Third(big bool) bool {
	var y float32
	y = 1
	if big {
		y = 1e21
	}
	return y/3*3 == y
}
