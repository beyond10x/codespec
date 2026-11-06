package expr_struct

// Point is a position.
type Point struct {
	X int
	Y int
}

// Make builds a point and reads its X.
func Make(x int) int {
	p := Point{X: x, Y: 0}
	return p.X
}
