// Package nohandler reads the environment through an extern the evaluator has no built-in
// handler for.
package nohandler

import "os"

// Mode returns the environment variable APP_MODE, or def when it is empty.
func Mode(def string) string {
	m := os.Getenv("APP_MODE")
	if m == "" {
		return def
	}
	return m
}
