// Package globaldefer holds a package-level function literal with a defer, which cir/1 does not
// express: the reader keeps it as a foreign statement inside the variable's initializer.
package globaldefer

var f = func() int {
	defer func() {}()
	return 1
}

// F calls the literal.
func F() int {
	return f()
}
