package type_array_float

import "fmt"

// Weight keeps three weights in a fixed-size array of 64-bit floats and three in one of 32-bit
// floats, sets them from float literals and float arithmetic, and formats the pair at i.
func Weight(i int) string {
	var w [3]float64
	w[0] = 0.1
	w[1] = w[0]*3 + 2.5e-1
	w[2] = w[1] / 4
	var h [3]float32
	h[0] = 0.1
	h[1] = h[0] * 3
	h[2] = h[1] - 1e-8
	return fmt.Sprintf("%v %v", w[i], h[i])
}

// Above tells whether the weight at i is greater than half.
func Above(i int) bool {
	var w [3]float64
	w[0] = 0.25
	w[1] = 0.5
	w[2] = 1e21
	return w[i] > 0.5
}
