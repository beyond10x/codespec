// Package readfile is the fixture of crates/cli/tests/barrier.rs: one function reading a file.
package readfile

import "os"

// Load reads the file at path.
func Load(path string) ([]byte, error) {
	return os.ReadFile(path)
}
