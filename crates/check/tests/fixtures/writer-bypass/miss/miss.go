// Package miss writes a URI into a writer with its writer method, and writes a name whose type
// has no writer method through its String method.
package miss

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

// Method is a request method.
type Method struct {
	Name string
}

// String renders m.
func (m Method) String() string {
	return strings.ToUpper(m.Name)
}

// Request is a method and a recipient.
type Request struct {
	Method Method
	To     *URI
}

// StartLineWrite writes the request line into w.
func (r *Request) StartLineWrite(w io.StringWriter) {
	w.WriteString(r.Method.String())
	w.WriteString(" ")
	r.To.StringWrite(w)
}
