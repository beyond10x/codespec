package type_float

import "fmt"

// Third divides a 64-bit and a 32-bit float by three and formats both quotients.
func Third(big bool) string {
	var x float64
	var y float32
	x = 1
	y = 1
	if big {
		x = 1e21
		y = 1e21
	}
	return fmt.Sprintf("%v %v", x/3, y/3)
}
