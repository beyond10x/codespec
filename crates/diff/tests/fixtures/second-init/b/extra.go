package probe

import "os"

func init() { _ = os.Getenv("MODE") }
