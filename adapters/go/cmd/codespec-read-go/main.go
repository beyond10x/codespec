// Command codespec-read-go reads one Go package directory and prints it as a cir/1 module on
// stdout. Types come from go/types; constructs cir/1 cannot express are kept as `foreign`
// statements and listed as refusals.
//
//	codespec-read-go <package dir>
//	codespec-read-go --spans <package dir>
//	codespec-read-go --describe
//	codespec-read-go --version
//
// --spans prints the module with where its names stand in the source (spans.go); --describe prints
// the reader's codespec-adapter/1 manifest (describe.go); --version prints the reader's version and
// the Go it was built with. Before reading, the reader refuses a host Go of a newer minor than its
// own (toolchain.go).
package main

import (
	"encoding/json"
	"fmt"
	"io"
	"os"
	"runtime"
)

const version = "0.14.0"

func main() {
	os.Exit(run(os.Args[1:], os.Stdout, os.Stderr))
}

func run(args []string, stdout, stderr io.Writer) int {
	if len(args) == 1 && args[0] == "--describe" {
		fmt.Fprint(stdout, describe())
		return 0
	}
	if len(args) == 1 && args[0] == "--version" {
		fmt.Fprint(stdout, versionLine())
		return 0
	}
	spans := len(args) == 2 && args[0] == "--spans"
	if len(args) != 1 && !spans {
		fmt.Fprintln(stderr, "usage: codespec-read-go [--spans] <package dir> | --describe | --version")
		return 2
	}
	if gap, ok := toolchainGap(hostGo(), runtime.Version()); ok {
		fmt.Fprintln(stderr, "codespec-read-go:", gap)
		return 1
	}
	if spans {
		if err := writeSpans(stdout, args[1]); err != nil {
			fmt.Fprintln(stderr, "codespec-read-go:", err)
			return 1
		}
		return 0
	}
	m, err := load(args[0])
	if err != nil {
		fmt.Fprintln(stderr, "codespec-read-go:", err)
		return 1
	}
	doc, err := encode(m)
	if err != nil {
		fmt.Fprintln(stderr, "codespec-read-go:", err)
		return 1
	}
	enc := json.NewEncoder(stdout)
	enc.SetEscapeHTML(false)
	if err := enc.Encode(doc); err != nil {
		fmt.Fprintln(stderr, "codespec-read-go:", err)
		return 1
	}
	return 0
}
