package expr_zero_struct

// case.cir.json is written by hand: the Go reader reads Point{} as a struct literal that sets no
// field. The C reader writes zero for a struct literal of zeros, (struct Point){0}.

// Point is a position.
type Point struct {
	X int
	Y int
}

// Sum gives the sum of the coordinates of the point (n, n), or of the zero point when n is not
// positive.
func Sum(n int) int {
	p := Point{X: n, Y: n}
	if n <= 0 {
		p = Point{}
	}
	return p.X + p.Y
}
