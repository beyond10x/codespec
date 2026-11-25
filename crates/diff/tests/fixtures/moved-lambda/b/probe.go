package main

import "strings"

// Shout upper-cases s.
// Only these two comment lines are new.
func Shout(s string) string {
	up := func(x string) string { return strings.ToUpper(x) }
	return up(s)
}

func main() { Shout("x") }
