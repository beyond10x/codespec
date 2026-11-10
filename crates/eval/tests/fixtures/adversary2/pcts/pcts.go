// Package pcts formats a string type with a String method through %s.
package pcts

import "fmt"

// S is a string with a String method.
type S string

// String names an S.
func (s S) String() string { return "S!" }

// Pct formats an S with %s.
func Pct() string {
	var s S
	s = "raw"
	return fmt.Sprintf("%s", s)
}
