// Package initorder declares a variable before the one its initializer reads; Go initializes
// package variables in dependency order.
package initorder

var a = b + 1
var b = 2

// A is a.
func A() int {
	return a
}
