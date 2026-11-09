// Package writer renders into writers that cannot fail: a strings.Builder, directly or through
// an io.StringWriter parameter every caller fills with one.
package writer

import (
	"io"
	"strings"
)

// Pair is a name and a value.
type Pair struct {
	K string
	V string
}

// String renders p.
func (p Pair) String() string {
	var b strings.Builder
	p.write(&b)
	b.WriteString(";")
	return b.String()
}

// Line renders p and a newline.
func (p Pair) Line() string {
	var b strings.Builder
	p.lineWrite(&b)
	return b.String()
}

func (p Pair) lineWrite(w io.StringWriter) {
	p.write(w)
	w.WriteString("\n")
}

func (p Pair) write(w io.StringWriter) {
	w.WriteString(p.K)
	w.WriteString("=")
	w.WriteString(p.V)
}
