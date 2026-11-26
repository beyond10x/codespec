package main

import (
	"bytes"
	"fmt"
	"os"
	"path/filepath"
	"runtime"
	"strings"
	"testing"
)

// stubGo puts a `go` first on PATH that answers `go env GOVERSION` with version and fails
// everything else, so no test reaches a real toolchain or the network through it.
func stubGo(t *testing.T, version string) {
	t.Helper()
	dir := t.TempDir()
	script := "#!/bin/sh\nif [ \"$1 $2\" = \"env GOVERSION\" ]; then echo '" + version + "'; exit 0; fi\nexit 1\n"
	if err := os.WriteFile(filepath.Join(dir, "go"), []byte(script), 0o755); err != nil {
		t.Fatal(err)
	}
	t.Setenv("PATH", dir+string(os.PathListSeparator)+os.Getenv("PATH"))
	t.Setenv("GOTOOLCHAIN", "local")
}

// buildMinor is the major and minor of the Go this test binary was built with.
func buildMinor(t *testing.T) (int, int) {
	t.Helper()
	var major, minor int
	if _, err := fmt.Sscanf(runtime.Version(), "go%d.%d", &major, &minor); err != nil {
		t.Skipf("the build Go %q has no release version", runtime.Version())
	}
	return major, minor
}

// randV2Module writes a module whose one package imports math/rand/v2, the package whose
// generic methods an older go/types refused in a newer standard library.
func randV2Module(t *testing.T) string {
	t.Helper()
	dir := t.TempDir()
	files := map[string]string{
		"go.mod":  "module example.com/dice\n\ngo 1.23\n",
		"dice.go": "package dice\n\nimport \"math/rand/v2\"\n\nfunc Roll() int {\n\treturn rand.IntN(6) + 1\n}\n",
	}
	for name, src := range files {
		if err := os.WriteFile(filepath.Join(dir, name), []byte(src), 0o644); err != nil {
			t.Fatal(err)
		}
	}
	return dir
}

func runReader(args ...string) (int, string, string) {
	var out, errb bytes.Buffer
	code := run(args, &out, &errb)
	return code, out.String(), errb.String()
}

// The host's Go one minor above the Go the reader was built with: the reader exits non-zero with
// one line naming both versions and the way out, and prints nothing on stdout, with and without
// --spans.
func TestToolchainAHostGoOfANewerMinorIsRefusedNamingBothVersionsAndReadingNothing(t *testing.T) {
	major, minor := buildMinor(t)
	host := fmt.Sprintf("go%d.%d.0", major, minor+1)
	stubGo(t, host+"-X:nodwarf5")
	dir := randV2Module(t)
	for _, args := range [][]string{{dir}, {"--spans", dir}} {
		code, out, errs := runReader(args...)
		if code == 0 {
			t.Fatalf("%v: exit 0 under a host %s, built with %s", args, host, runtime.Version())
		}
		if out != "" {
			t.Fatalf("%v: printed %q on stdout, want nothing read", args, out)
		}
		lines := strings.Split(strings.TrimSuffix(errs, "\n"), "\n")
		if len(lines) != 1 {
			t.Fatalf("%v: want one line on stderr, got %q", args, errs)
		}
		build := strings.FieldsFunc(runtime.Version(), func(r rune) bool { return r == ' ' || r == '-' })[0]
		for _, want := range []string{host, build, "GOTOOLCHAIN=" + build, fmt.Sprintf("go%d.%d", major, minor+1)} {
			if !strings.Contains(lines[0], want) {
				t.Fatalf("%v: the refusal %q does not name %q", args, lines[0], want)
			}
		}
		if strings.Contains(lines[0], "X:nodwarf5") {
			t.Fatalf("%v: the refusal %q keeps the GOVERSION suffix", args, lines[0])
		}
	}
}

// --describe reads no package, so a newer host Go does not stop it.
func TestToolchainDescribeIsNotRefusedUnderANewerHostGo(t *testing.T) {
	major, minor := buildMinor(t)
	stubGo(t, fmt.Sprintf("go%d.%d.0", major, minor+1))
	code, out, errs := runReader("--describe")
	if code != 0 || out != describe() || errs != "" {
		t.Fatalf("--describe: exit %d, stdout %q, stderr %q", code, out, errs)
	}
}

// A newer patch release of the same minor reads: the refusal compares minor versions.
func TestToolchainANewerPatchOfTheSameMinorReads(t *testing.T) {
	major, minor := buildMinor(t)
	stubGo(t, fmt.Sprintf("go%d.%d.99-X:nodwarf5", major, minor))
	code, out, errs := runReader(randV2Module(t))
	if code != 0 || !strings.Contains(out, `"Roll"`) {
		t.Fatalf("exit %d, stdout %q, stderr %q", code, out, errs)
	}
}

// A module importing math/rand/v2 reads under the Go the tests run with, with no refusal of
// the toolchain and no type error.
func TestToolchainAModuleImportingMathRandV2Reads(t *testing.T) {
	t.Setenv("GOTOOLCHAIN", "local")
	code, out, errs := runReader(randV2Module(t))
	if code != 0 || errs != "" {
		t.Fatalf("exit %d, stderr %q", code, errs)
	}
	if !strings.Contains(out, `"Roll"`) {
		t.Fatalf("Roll not read: %s", out)
	}
}

// The gate and the release build with one Go: every setup-go step of check.yml and
// release-build.yml names the same explicit go-version, and none takes it from go.mod (whose
// `go 1.23` gave the release a go/types older than the host's standard library).
func TestToolchainEverySetupGoStepNamesOneGoVersion(t *testing.T) {
	versions := map[string][]string{}
	steps := 0
	for _, wf := range []string{"check.yml", "release-build.yml"} {
		src, err := os.ReadFile(filepath.Join("..", "..", "..", "..", ".github", "workflows", wf))
		if err != nil {
			t.Fatal(err)
		}
		for _, step := range strings.Split(string(src), "- uses: ")[1:] {
			if !strings.HasPrefix(step, "actions/setup-go@") {
				continue
			}
			steps++
			if end := strings.Index(step, "\n      - "); end >= 0 {
				step = step[:end]
			}
			if strings.Contains(step, "go-version-file") {
				t.Errorf("%s: a setup-go step takes its Go from a file:\n%s", wf, step)
			}
			v := ""
			for _, line := range strings.Split(step, "\n") {
				if rest, ok := strings.CutPrefix(strings.TrimSpace(line), "go-version:"); ok {
					v = strings.Trim(strings.TrimSpace(rest), `"'`)
				}
			}
			versions[v] = append(versions[v], wf)
		}
	}
	if steps != 3 {
		t.Errorf("want the 3 setup-go steps (check.yml 1, release-build.yml 2), found %d", steps)
	}
	if len(versions) != 1 || versions[""] != nil {
		t.Errorf("the setup-go steps name more than one go-version, or none: %v", versions)
	}
}

// --version prints the reader's version and the Go it was built with.
func TestToolchainVersionPrintsTheGoTheReaderWasBuiltWith(t *testing.T) {
	code, out, errs := runReader("--version")
	want := "codespec-read-go " + version + " built with " + runtime.Version() + "\n"
	if code != 0 || out != want || errs != "" {
		t.Fatalf("--version: exit %d, stdout %q (want %q), stderr %q", code, out, want, errs)
	}
}

func TestToolchainGapComparesMinorVersionsAndCutsSuffixes(t *testing.T) {
	for _, c := range []struct {
		host, build string
		gap         bool
	}{
		{"go1.27.1-X:nodwarf5", "go1.23.12", true},
		{"go1.27.1", "go1.27.4", false},
		{"go1.27.4", "go1.27.1 X:nodwarf5", false},
		{"go1.28rc1", "go1.27.4", true},
		{"go1.28", "go1.27.4", true},
		{"go2.0.0", "go1.27.4", true},
		{"go1.26.11", "go1.27.4", false},
		{"devel go1.28-abcdef", "go1.27.4", false},
		{"go1.28.0", "devel go1.28-abcdef", false},
		{"", "go1.27.4", false},
	} {
		if _, got := toolchainGap(c.host, c.build); got != c.gap {
			t.Errorf("toolchainGap(%q, %q) = %v, want %v", c.host, c.build, got, c.gap)
		}
	}
}
