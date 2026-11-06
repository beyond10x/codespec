// Command codespec-run-go builds one written Go package with the Go toolchain, makes the calls of
// a codespec-calls/1 document against it and prints one line per call in the harness encoding.
//
//	codespec-run-go [-build <dir>] <package dir> <calls.json>
//
// It is the conformance kit's reference runner, applied to any package: the driver it builds is
// the kit's own conformance/driver/go/main.go (embedded as driver.go.txt and checked byte-equal to
// it), and the calls.go it generates is the one codespec-conformance generates for the same calls,
// derived here from the package's go/types signatures instead of its cir/1 module. The encoding of
// arguments and output lines is the crate doc of crates/conformance.
//
// The build directory (-build, emptied first; else a temporary directory removed afterwards) holds
// go.mod, the driver as main.go, calls.go and the package's .go files other than tests under
// subject/.
//
// Exit status, as codespec-run-rust's:
//
//	0  every call made and printed
//	2  refused: calls.json not as read_calls in crates/conformance reads it, a call that does not
//	   fit the package (no such exported function, a wrong argument, a parameter or result type the
//	   driver cannot pass or print, unsigned integers among them), a value the driver cannot print;
//	   a usage error too
//	1  failed: the package does not parse or type-check, the build fails, or the program fails
package main

import (
	_ "embed"
	"errors"
	"flag"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
)

//go:embed driver.go.txt
var driver []byte

func main() {
	fs := flag.NewFlagSet("codespec-run-go", flag.ContinueOnError)
	build := fs.String("build", "", "the build directory, emptied first (default: a temporary directory)")
	if err := fs.Parse(os.Args[1:]); err != nil || fs.NArg() != 2 {
		fmt.Fprintln(os.Stderr, "usage: codespec-run-go [-build <dir>] <package dir> <calls.json>")
		os.Exit(2)
	}
	if err := run(fs.Arg(0), fs.Arg(1), *build); err != nil {
		fmt.Fprintln(os.Stderr, "codespec-run-go:", err)
		var r refused
		if errors.As(err, &r) {
			os.Exit(2)
		}
		os.Exit(1)
	}
}

// refused is an error that exits 2: the input does not fit, as opposed to a failure.
type refused struct{ error }

func (r refused) Unwrap() error { return r.error }

func run(dir, callsPath, build string) error {
	calls, err := readCalls(callsPath)
	if err != nil {
		return refused{err}
	}
	pkg, files, err := load(dir)
	if err != nil {
		return err
	}
	src, err := callsGo(pkg, calls)
	if err != nil {
		return refused{fmt.Errorf("%s: %w", callsPath, err)}
	}
	if build == "" {
		tmp, err := os.MkdirTemp("", "codespec-run-go-")
		if err != nil {
			return err
		}
		defer os.RemoveAll(tmp)
		build = tmp
	} else if err := os.RemoveAll(build); err != nil {
		return err
	}
	build, err = filepath.Abs(build)
	if err != nil {
		return err
	}
	subject := filepath.Join(build, "subject")
	if err := os.MkdirAll(subject, 0o755); err != nil {
		return err
	}
	write := map[string][]byte{
		filepath.Join(build, "go.mod"):   []byte("module codespec.conformance/case\n\ngo 1.22\n"),
		filepath.Join(build, "main.go"):  driver,
		filepath.Join(build, "calls.go"): []byte(src),
	}
	for _, f := range files {
		b, err := os.ReadFile(f)
		if err != nil {
			return err
		}
		write[filepath.Join(subject, filepath.Base(f))] = b
	}
	for p, b := range write {
		if err := os.WriteFile(p, b, 0o644); err != nil {
			return err
		}
	}
	bin := filepath.Join(build, "run")
	cmd := exec.Command("go", "build", "-o", bin, ".")
	cmd.Dir = build
	cmd.Env = append(os.Environ(), "GOWORK=off", "GOFLAGS=")
	if out, err := cmd.CombinedOutput(); err != nil {
		return fmt.Errorf("go build: %v\n%s", err, strings.TrimSpace(string(out)))
	}
	prog := exec.Command(bin)
	prog.Stdout = os.Stdout
	prog.Stderr = os.Stderr
	if err := prog.Run(); err != nil {
		var exit *exec.ExitError
		if errors.As(err, &exit) && exit.ExitCode() == 2 {
			return refused{fmt.Errorf("the program met a value it cannot print: %v", err)}
		}
		return fmt.Errorf("the program: %v", err)
	}
	return nil
}
