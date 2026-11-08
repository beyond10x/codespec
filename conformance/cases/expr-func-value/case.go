package expr_func_value

// Op maps an int to an int.
type Op func(int) int

// Counter adds its step.
type Counter struct {
	Step int
}

// Add gives n plus the step.
func (c Counter) Add(n int) int {
	return n + c.Step
}

func double(n int) int {
	return n * 2
}

// Compose applies the function double, then the method Add of a counter of step k, to n: one
// module function and one method used as values.
func Compose(n int, k int) int {
	f := Op(double)
	c := Counter{Step: k}
	g := c.Add
	return g(f(n))
}
