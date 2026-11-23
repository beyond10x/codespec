// Package deferred renders into an io.StringWriter that one caller fills with a strings.Builder
// and a deferred caller with a file, which can fail.
package deferred

import (
	"io"
	"os"
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
	return b.String()
}

// Save renders p into f when it returns.
func (p Pair) Save(f *os.File) {
	defer p.write(f)
}

func (p Pair) write(w io.StringWriter) {
	w.WriteString(p.K)
	w.WriteString("=")
	w.WriteString(p.V)
}
