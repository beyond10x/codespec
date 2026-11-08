package decl_var

// greeting starts every greeting.
var greeting = "hello, "

// hits counts the calls of Greet; it starts at its zero value.
var hits int

// Greet counts the call and greets name.
func Greet(name string) string {
	hits = hits + 1
	return greeting + name
}

// Calls is how many times Greet ran.
func Calls() int {
	return hits
}
