package decl_union

// case.cir.json is written by hand: Go has no tagged union. The Rust reader reads a Rust enum
// whose variants carry fields as one. Its case.cir.json declares
//
//	Shape: Circle{Radius int} | Rect{W int, H int} | Label{Text string}
//
// and cir/1 has no expression that builds or inspects a union value, so the case checks that a
// writer writes the declaration into a module that builds, beside a function it can call.

// Echo returns its argument.
func Echo(n int) int {
	return n
}
