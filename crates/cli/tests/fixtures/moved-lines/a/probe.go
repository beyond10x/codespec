package probe

import "os"

func Load() string {
	v := os.Getenv("MODE")
	if v == "" {
		v = os.Getenv("DEFAULT_MODE")
	}
	return v
}
