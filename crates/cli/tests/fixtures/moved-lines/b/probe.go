package probe

import "os"

// Load reads the mode from the environment, and the default mode
// when the mode is not set. This revision only adds these comment
// lines and the blank line below them.

func Load() string {
	v := os.Getenv("MODE")
	if v == "" {
		v = os.Getenv("DEFAULT_MODE")
	}
	return v
}
