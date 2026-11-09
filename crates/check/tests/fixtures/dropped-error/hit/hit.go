// Package hit removes a file and drops the error.
package hit

import "os"

// Clean removes the file at path.
func Clean(path string) {
	os.Remove(path)
}
