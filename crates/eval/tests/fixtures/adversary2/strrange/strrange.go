// Package strrange ranges over a long string and returns at its first byte.
package strrange

import "strings"

// First returns the index of the first rune of a string of 2^28 bytes.
func First() int {
	s := strings.Repeat("a", 1<<28)
	for i := range s {
		return i
	}
	return -1
}
