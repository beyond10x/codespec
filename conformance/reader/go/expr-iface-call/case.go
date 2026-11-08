package expr_iface_call

import (
	"io"
	"strings"
)

func emit(w io.StringWriter, s string) {
	w.WriteString(s)
}

// Twice writes s twice into a builder of the standard library through io.StringWriter, an
// interface of another package, and gives what it holds.
func Twice(s string) string {
	var b strings.Builder
	emit(&b, s)
	emit(&b, s)
	return b.String()
}
