// Command codespec-read-go reads one Go package directory and prints it as a cir/1 module on
// stdout. Types come from go/types; constructs cir/1 cannot express are kept as `foreign`
// statements and listed as refusals.
//
//	codespec-read-go <package dir>
//	codespec-read-go --spans <package dir>
//	codespec-read-go --describe
//
// --spans prints the module with where its names stand in the source (spans.go); --describe prints
// the reader's codespec-adapter/1 manifest (describe.go).
package main

import (
	"encoding/json"
	"fmt"
	"os"
)

const version = "0.13.0"

func main() {
	if len(os.Args) == 2 && os.Args[1] == "--describe" {
		fmt.Print(describe())
		return
	}
	if len(os.Args) == 3 && os.Args[1] == "--spans" {
		if err := writeSpans(os.Stdout, os.Args[2]); err != nil {
			fmt.Fprintln(os.Stderr, "codespec-read-go:", err)
			os.Exit(1)
		}
		return
	}
	if len(os.Args) != 2 {
		fmt.Fprintln(os.Stderr, "usage: codespec-read-go [--spans] <package dir> | --describe")
		os.Exit(2)
	}
	m, err := load(os.Args[1])
	if err != nil {
		fmt.Fprintln(os.Stderr, "codespec-read-go:", err)
		os.Exit(1)
	}
	doc, err := encode(m)
	if err != nil {
		fmt.Fprintln(os.Stderr, "codespec-read-go:", err)
		os.Exit(1)
	}
	enc := json.NewEncoder(os.Stdout)
	enc.SetEscapeHTML(false)
	if err := enc.Encode(doc); err != nil {
		fmt.Fprintln(os.Stderr, "codespec-read-go:", err)
		os.Exit(1)
	}
}
