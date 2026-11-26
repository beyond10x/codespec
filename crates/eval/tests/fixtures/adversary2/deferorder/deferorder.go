// Package deferorder probes when a deferred call takes its operands and when it runs: a value
// receiver copied at the defer, a pointer receiver read at the call, defers in a loop, an early
// return, a function value reassigned after the defer, a call through an interface, a method
// value, a call in an argument, a defer inside a deferred literal, and a result taken before the
// deferred calls run.
package deferorder

import "fmt"

// C is a struct with a value and a pointer method.
type C struct{ n int }

// Show prints the copy it was called on.
func (c C) Show() { fmt.Println("val", c.n) }

// PShow prints what its pointer points to when it runs.
func (c *C) PShow() { fmt.Println("ptr", c.n) }

// Shower shows.
type Shower interface{ Show() }

func show(s string, v int) { fmt.Println(s, v) }

// Recv defers a value-receiver and a pointer-receiver call on one local, then changes it.
func Recv() int {
	c := C{1}
	defer c.Show()
	defer c.PShow()
	c.n = 2
	return c.n
}

// Loop defers two calls per iteration.
func Loop() int {
	for i := 0; i < 3; i++ {
		defer show("loop", i)
		defer func() { fmt.Println("lit", i) }()
	}
	return 9
}

// Early returns from inside an if, after a second defer there.
func Early(n int) int {
	defer show("early", n)
	if n > 0 {
		defer show("pos", n)
		return n * 2
	}
	for {
		defer show("in-for", n)
		break
	}
	return -1
}

// FuncVal defers a function value, reassigns it and defers it again.
func FuncVal() int {
	f := func(s string) { fmt.Println("f1", s) }
	defer f("a")
	f = func(s string) { fmt.Println("f2", s) }
	defer f("b")
	return 0
}

// Iface defers a call through an interface, then gives the interface another value.
func Iface() int {
	var s Shower = C{5}
	defer s.Show()
	s = C{6}
	return 1
}

// MethodValue defers a bound method value of a pointer, then changes what it points to.
func MethodValue() int {
	c := &C{1}
	m := c.PShow
	defer m()
	c.n = 7
	return 0
}

// ArgCall defers a call whose argument is a call with an effect.
func ArgCall() int {
	n := 1
	next := func() int {
		n++
		return n
	}
	defer show("arg", next())
	fmt.Println("body", n)
	return n
}

// Nested defers a literal that defers a call of its own.
func Nested() int {
	defer func() {
		defer show("inner", 1)
		fmt.Println("outer")
	}()
	return 3
}

// Taken returns a value that a deferred literal changes after the return took it.
func Taken() int {
	x := 1
	p := &x
	defer func() { *p = 5 }()
	return *p
}

// Rec defers in each level of a recursion.
func Rec(n int) int {
	if n == 0 {
		return 0
	}
	defer show("rec", n)
	return Rec(n-1) + n
}
