package main

import (
	"go/build"
	"os/exec"
	"strings"
)

// A binary built with -trimpath carries no GOROOT, and the source importer then cannot find the
// standard library. Ask the installed Go toolchain where it is, as `go build` would.
func ensureGOROOT() {
	if build.Default.GOROOT != "" {
		return
	}
	out, err := exec.Command("go", "env", "GOROOT").Output()
	if err != nil {
		return
	}
	build.Default.GOROOT = strings.TrimSpace(string(out))
}
