// Package onlyb is imported by cmd/b only; its init reads the environment.
package onlyb

import "os"

func init() { hook() }

func hook() { _ = os.Getenv("PROBE") }

// Run does nothing.
func Run() {}
