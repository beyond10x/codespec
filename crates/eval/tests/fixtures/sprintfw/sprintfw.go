// Package sprintfw formats an error with %w through Sprintf, where only Errorf knows the verb.
package sprintfw

import (
	"errors"
	"fmt"
)

var errBoom = errors.New("boom")

// Show is Sprintf's text for %w.
func Show() string {
	return fmt.Sprintf("x %w", errBoom)
}
