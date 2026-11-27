package main

import (
	"fmt"
	"os"
	"path/filepath"
	"runtime"
	"strings"
	"testing"
)

// The go-version of every setup-go step is not older, by minor, than the Go the tests run with:
// the unit's own test (TestToolchainEverySetupGoStepNamesOneGoVersion) accepts any one version,
// so "1.23.12" in all three steps, the Go whose go/types the story exists to leave, passes it.
func TestAdversaryToolchainSetupGoIsNotOlderThanTheGoTheTestsRunWith(t *testing.T) {
	major, minor := buildMinor(t)
	seen := 0
	for _, wf := range []string{"check.yml", "release-build.yml"} {
		src, err := os.ReadFile(filepath.Join("..", "..", "..", "..", ".github", "workflows", wf))
		if err != nil {
			t.Fatal(err)
		}
		for _, line := range strings.Split(string(src), "\n") {
			rest, ok := strings.CutPrefix(strings.TrimSpace(line), "go-version:")
			if !ok {
				continue
			}
			seen++
			v := strings.Trim(strings.TrimSpace(rest), `"'`)
			var m, n int
			if _, err := fmt.Sscanf("go"+v, "go%d.%d", &m, &n); err != nil {
				t.Errorf("%s: go-version %q has no major and minor", wf, v)
				continue
			}
			if m < major || (m == major && n < minor) {
				t.Errorf("%s: setup-go builds with go%s, older than the go%d.%d the tests run with (%s)", wf, v, major, minor, runtime.Version())
			}
		}
	}
	if seen != 3 {
		t.Errorf("want 3 go-version lines, found %d", seen)
	}
}

// The refusal's first way out names the host's release: "built with go<host major>.<host minor>
// or later". The unit's test looks for "go<host major>.<host minor+1>" as a substring of the
// whole line, which the host's own version in the line's first half already contains.
func TestAdversaryToolchainRefusalAsksForAReaderBuiltWithTheHostsGo(t *testing.T) {
	gap, ok := toolchainGap("go1.29.2-X:nodwarf5", "go1.27.4")
	if !ok {
		t.Fatal("no gap between go1.29.2 and go1.27.4")
	}
	want := "use a codespec-read-go built with go1.29 or later, or run it with GOTOOLCHAIN=go1.27.4"
	if !strings.HasSuffix(gap, want) {
		t.Fatalf("the refusal %q does not end with %q", gap, want)
	}
}
