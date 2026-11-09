// Package miss removes a file and returns the error; a print's error is ignored on purpose.
package miss

import (
	"fmt"
	"os"
)

// Clean removes the file at path.
func Clean(path string) error {
	err := os.Remove(path)
	if err != nil {
		fmt.Fprintln(os.Stderr, "clean:", err)
	}
	return err
}
