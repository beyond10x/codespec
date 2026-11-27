package main

import (
	"fmt"
	"os/exec"
	"runtime"
	"strings"
)

// The reader type-checks the host's standard library with the go/types it was built with. A
// go/types older than that library fails package by package (go1.23 on go1.27's math/rand/v2:
// "method must have no type parameters"), so a host Go of a newer minor is refused before
// anything is read.

// releaseOf cuts a Go version to its release: "go1.27.1-X:nodwarf5" and "go1.27.1 X:nodwarf5"
// are "go1.27.1".
func releaseOf(v string) string {
	if i := strings.IndexAny(v, " -"); i >= 0 {
		return v[:i]
	}
	return v
}

// minorOf reads the major and minor of a Go release version; a devel or empty version has none.
func minorOf(v string) (int, int, bool) {
	var major, minor int
	if _, err := fmt.Sscanf(releaseOf(v), "go%d.%d", &major, &minor); err != nil {
		return 0, 0, false
	}
	return major, minor, true
}

// toolchainGap reports the refusal when the host Go is of a newer minor than the build Go. A
// version without a major and minor (devel, or no go on PATH) is no gap.
func toolchainGap(host, build string) (string, bool) {
	hMajor, hMinor, ok := minorOf(host)
	if !ok {
		return "", false
	}
	bMajor, bMinor, ok := minorOf(build)
	if !ok || hMajor < bMajor || (hMajor == bMajor && hMinor <= bMinor) {
		return "", false
	}
	h, b := releaseOf(host), releaseOf(build)
	return fmt.Sprintf("the host's Go is %s, newer than %s, the Go this reader was built with, "+
		"whose go/types cannot read a newer standard library: use a codespec-read-go built with "+
		"go%d.%d or later, or run it with GOTOOLCHAIN=%s", h, b, hMajor, hMinor, b), true
}

// hostGo is the version of the go on PATH, as `go env GOVERSION` prints it; empty when there is
// none. It runs in the working directory, as ensureGOROOT does.
func hostGo() string {
	out, err := exec.Command("go", "env", "GOVERSION").Output()
	if err != nil {
		return ""
	}
	return strings.TrimSpace(string(out))
}

func versionLine() string {
	return "codespec-read-go " + version + " built with " + runtime.Version() + "\n"
}
