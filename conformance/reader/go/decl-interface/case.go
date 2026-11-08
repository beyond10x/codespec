package decl_interface

// Shape has an area.
type Shape interface {
	Area() int
}

type Square struct {
	side int
}

func (s Square) Area() int {
	return s.side * s.side
}

type Rect struct {
	w int
	h int
}

func (r Rect) Area() int {
	return r.w * r.h
}

func area(s Shape) int {
	return s.Area()
}

// Areas sums, through Shape, the area of a square of side n and of a rectangle n by 2.
func Areas(n int) int {
	return area(Square{side: n}) + area(Rect{w: n, h: 2})
}
