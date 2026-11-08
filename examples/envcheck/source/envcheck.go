// Package envcheck builds a service banner from its environment.
package envcheck

import (
	"fmt"
	"os"
	"strconv"
	"strings"
)

// Mode is the run mode from APP_MODE, upper-cased, or "DEV" when it is unset or blank.
func Mode() string {
	m := strings.TrimSpace(os.Getenv("APP_MODE"))
	if m == "" {
		return "DEV"
	}
	return strings.ToUpper(m)
}

// Label is the service name, lower-cased, and its port.
func Label(name string, port int) string {
	return strings.ToLower(name) + ":" + strconv.Itoa(port)
}

// Banner is the mode and the label.
func Banner(name string, port int) string {
	return Mode() + " " + Label(name, port)
}

// Announce prints the banner.
func Announce(name string, port int) {
	fmt.Println(Banner(name, port))
}

// Save writes the banner to a file. os.WriteFile takes an fs.FileMode, a type cir/1 has none for.
func Save(path string, name string, port int) error {
	return os.WriteFile(path, []byte(Banner(name, port)), 0o644)
}
