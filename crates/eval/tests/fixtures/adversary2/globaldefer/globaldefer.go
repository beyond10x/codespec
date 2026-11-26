// Package globaldefer holds a package-level function literal that starts a goroutine, which
// cir/1 does not express (the directory keeps its name from when the literal deferred a call):
// the reader keeps it as a foreign statement inside the variable's initializer.
package globaldefer

var f = func() int {
	go func() {}()
	return 1
}

// F calls the literal.
func F() int {
	return f()
}
