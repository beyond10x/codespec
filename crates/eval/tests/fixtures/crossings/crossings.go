// Package crossings moves values of two int types into interface slots by a two-valued
// assignment, where Go keeps each value's type.
package crossings

// A is an int type.
type A int

// B is another int type.
type B int

func two() (A, B) { return 1, 1 }

// Pair assigns a two-valued call to two any variables declared before it and compares them.
func Pair() bool {
	var x any
	var y any
	x, y = two()
	return x == y
}
