// Package advsession holds what the adversary's session cases call.
package advsession

// Counter counts the calls of Inc.
type Counter struct{ n int }

// NewCounter returns a zero Counter, by value.
func NewCounter() Counter { return Counter{} }

// Inc adds one to c.
func (c *Counter) Inc() { c.n++ }

// Get returns the count.
func (c Counter) Get() int { return c.n }

// Name returns a string.
func Name() string { return "ann" }

// Div divides a by b.
func Div(a, b int) int { return a / b }

// Twice doubles n.
func Twice(n int) int { return n * 2 }

// Msg returns the message of e, or "nil".
func Msg(e error) string {
	if e == nil {
		return "nil"
	}
	return e.Error()
}

var rendered int

// Loud is an error whose Error method counts its calls.
type Loud struct{}

// Error counts the call.
func (Loud) Error() string {
	rendered++
	return "loud"
}

// Fail returns a Loud error; nothing in it calls Error.
func Fail() error { return Loud{} }

// Rendered returns how often Loud.Error ran.
func Rendered() int { return rendered }
