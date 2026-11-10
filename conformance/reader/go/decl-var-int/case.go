package decl_var_int

// start is where every count starts.
var start = 10

// hits counts what Hit was given; it starts at its zero value.
var hits int

// Hit adds n to the count and gives the start plus the count.
func Hit(n int) int {
	hits = hits + n
	return start + hits
}

// Calls is the count.
func Calls() int {
	return hits
}
