package main

import "strings"

func Shout(s string) string {
	up := func(x string) string { return strings.ToUpper(x) }
	return up(s)
}

func main() { Shout("x") }
