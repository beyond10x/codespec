// Package nohandler reads the process id through an extern the evaluator has no built-in
// handler for.
package nohandler

import "os"

// Pid returns the process id, or def when it is not positive.
func Pid(def int) int {
	p := os.Getpid()
	if p <= 0 {
		return def
	}
	return p
}
