// Package library has an exported function, the helper it calls, and a helper nothing calls.
package library

// Greet greets n.
func Greet(n string) string {
	return greeting() + n
}

func greeting() string {
	return "hello, "
}

func stale() string {
	return "hi, "
}
