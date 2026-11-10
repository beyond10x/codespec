// Package fieldstr prints a struct whose unexported field has a String method, which fmt does
// not call: it cannot reach an unexported field's methods.
package fieldstr

import "fmt"

// C prints as "c!".
type C int

// String names a C.
func (c C) String() string { return "c!" }

// W holds a C unexported and one exported.
type W struct {
	c C
	D C
}

// Show prints a W.
func Show() {
	fmt.Println(W{c: 1, D: 2})
}
