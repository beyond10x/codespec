// Package vari passes concrete values where interfaces are expected by paths other than the
// ones the first adversary pass named: a variadic ...any, a function literal's parameters, a
// two-valued call spread over two parameters, and a two-valued call returned as two any.
package vari

import "fmt"

// A is an int with a String method.
type A int

// B is another int type.
type B int

// String names an A.
func (a A) String() string { return "A!" }

func same(xs ...any) bool { return xs[0] == xs[1] }

func text(xs ...any) string { return fmt.Sprintf("%v", xs[0]) }

// Same compares an A and a B, both 1, passed through a variadic ...any.
func Same() bool {
	var a A
	a = 1
	var b B
	b = 1
	return same(a, b)
}

// Text prints an A passed through a variadic ...any.
func Text() string {
	var a A
	a = 1
	return text(a)
}

// Closure compares an A and a B passed to a function literal taking two any.
func Closure() bool {
	var a A
	a = 1
	var b B
	b = 1
	f := func(x, y any) bool { return x == y }
	return f(a, b)
}

func two() (A, B) { return 1, 1 }

func pair(x, y any) bool { return x == y }

// Spread passes a two-valued call to two any parameters.
func Spread() bool { return pair(two()) }

func ret() (any, any) { return two() }

// Ret returns a two-valued call from a function returning two any.
func Ret() bool {
	x, y := ret()
	return x == y
}
