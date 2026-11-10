// Package bldcopy writes to a copy of a strings.Builder, which Go's Builder panics on.
package bldcopy

import "strings"

// Copy writes to a copy of a non-zero Builder.
func Copy() string {
	var b strings.Builder
	b.WriteString("x")
	c := b
	c.WriteString("y")
	return c.String()
}
