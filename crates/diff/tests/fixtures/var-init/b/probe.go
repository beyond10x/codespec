package probe

import "os"

var Mode = os.Getenv("MODE")

func Keep() string { return Mode }
