package type_unit

import "fmt"

// case.cir.json is written by hand: the Go reader types a call of a function without results as
// an empty tuple, and fmt.Println as the tuple (int, error) it returns. The Rust and Java readers
// type both as unit, the value of a call that gives none.

func say(s string) {
	fmt.Println(s)
}

// Greet calls a function that returns nothing.
func Greet(name string) {
	say(name)
}
