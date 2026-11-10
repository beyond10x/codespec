// Package initfn sets a package variable in an init function.
package initfn

var x int

func init() { x = 5 }

// X gives what init stored.
func X() int { return x }
