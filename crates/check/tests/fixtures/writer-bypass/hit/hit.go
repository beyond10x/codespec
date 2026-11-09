// Package hit writes a URI into a writer through its String method, which builds the string
// with the writer method the caller bypasses.
package hit

import (
	"io"
	"strings"
)

// URI is a host.
type URI struct {
	Host string
}

// String renders u.
func (u *URI) String() string {
	var b strings.Builder
	u.StringWrite(&b)
	return b.String()
}

// StringWrite renders u into w.
func (u *URI) StringWrite(w io.StringWriter) {
	w.WriteString("sip:")
	w.WriteString(u.Host)
}

// Request is a method and a recipient.
type Request struct {
	Method string
	To     *URI
}

// StartLineWrite writes the request line into w.
func (r *Request) StartLineWrite(w io.StringWriter) {
	w.WriteString(r.Method)
	w.WriteString(" ")
	w.WriteString(r.To.String())
}
