package expr_method

// Acc accumulates.
type Acc struct {
	N int
}

// Add adds n.
func (a *Acc) Add(n int) {
	a.N = a.N + n
}

// Sum adds two values through a method.
func Sum(x int, y int) int {
	a := &Acc{}
	a.Add(x)
	a.Add(y)
	return a.N
}
