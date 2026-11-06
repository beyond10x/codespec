package intrinsic_sprintf

import "fmt"

// Label formats a name and a count.
func Label(name string, n int) string {
	return fmt.Sprintf("%s=%d", name, n)
}
