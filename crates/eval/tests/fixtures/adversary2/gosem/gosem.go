// Package gosem holds the Go semantics of the maintainer review of 2026-11-23, findings E1 to
// E6: each function's Go answer is quoted in crates/eval/tests/adversary2.rs.
package gosem

import "fmt"

// LenNilArrayPtr is len of a nil *[3]int: the type's length, 3.
func LenNilArrayPtr() int {
	var p *[3]int
	return len(p)
}

type box struct{ arr [3]int }

func next(c *int) *box {
	*c++
	return &box{}
}

func nilArr(c *int) *[3]int {
	*c++
	return nil
}

func idx(c *int) int {
	*c++
	return 1
}

// LenCallField is len of an array field reached through a call: Go calls it, 13.
func LenCallField() int {
	c := 0
	n := len(next(&c).arr)
	return c*10 + n
}

// LenCallNilPtr is len of a nil *[3]int a call returns: Go calls it and gives 3, 13.
func LenCallNilPtr() int {
	c := 0
	n := len(nilArr(&c))
	return c*10 + n
}

// LenCallIndex is len of an array element whose index is a call: Go calls it, 13.
func LenCallIndex() int {
	c := 0
	var grid [2][3]int
	n := len(grid[idx(&c)])
	return c*10 + n
}

// IfaceMapEq compares an any holding a map with itself: Go panics.
func IfaceMapEq() bool {
	var a any
	a = map[string]int{}
	b := a
	return a == b
}

// IfaceSliceNe compares two any holding nil slices with !=: Go panics.
func IfaceSliceNe() bool {
	var s []int
	var a any
	a = s
	var b any
	b = s
	return a != b
}

// IfaceSwitch switches on an any holding a nil slice with a case of the same type: Go panics.
func IfaceSwitch() int {
	var s []int
	var a any
	a = s
	var b any
	b = s
	switch a {
	case b:
		return 1
	}
	return 0
}

// IfaceDiffTypes compares an any holding a map with one holding an int: false.
func IfaceDiffTypes() bool {
	var a any
	a = map[string]int{}
	var b any
	b = 1
	return a == b
}

// IfaceInts compares two any holding equal ints: true.
func IfaceInts() bool {
	var a any
	a = 3
	var b any
	b = 3
	return a == b
}

// RangeNilChan ranges over a nil channel: Go blocks forever.
func RangeNilChan() int {
	var c chan int
	n := 0
	for v := range c {
		n += v
	}
	return 17 + n
}

// RangeNilSlice ranges over a nil slice: no iteration, 17.
func RangeNilSlice() int {
	var s []int
	n := 0
	for v := range s {
		n += v
	}
	return 17 + n
}

// Shadow declares a local x in a closure that read the outer x: 31.
func Shadow() int {
	x := 1
	f := func() int {
		y := x
		x := 2
		return y + x
	}
	return f()*10 + x
}

// Assign assigns the outer x in a closure: 5.
func Assign() int {
	x := 1
	f := func() {
		x = 5
	}
	f()
	return x
}

type counter struct{ n *int }

// String reads the counter when it is formatted.
func (c counter) String() string {
	if *c.n > 0 {
		return "after"
	}
	return "before"
}

func bump(n *int) int {
	*n++
	return *n
}

// PrintOrder prints a counter and then bumps it in the next argument: Go prints "after 1".
func PrintOrder() int {
	n := 0
	fmt.Println(counter{n: &n}, bump(&n))
	return n
}
