package probe

import "os"

func Mode() string { return os.Getenv("MODE") }
