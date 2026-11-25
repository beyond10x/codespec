// Package shared is imported by both programs, for its init only.
package shared

import "os"

func init() { _ = os.Getpid() }
