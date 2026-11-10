// Package twosites reads one variable of the environment at two call sites of one extern.
package twosites

import "os"

// Pair gives the variable MODE read at two sites, joined by a slash.
func Pair() string {
	first := os.Getenv("MODE")
	second := os.Getenv("MODE")
	return first + "/" + second
}
