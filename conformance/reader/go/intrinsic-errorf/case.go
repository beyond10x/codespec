package intrinsic_errorf

import "fmt"

// Validate refuses a negative.
func Validate(n int) error {
	if n < 0 {
		return fmt.Errorf("negative: %d", n)
	}
	return nil
}
