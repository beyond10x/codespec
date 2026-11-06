// Command codespec-write-go reads one cir/1 module on stdin and writes it as a Go package into
// the output directory: one file, <module name>.go. The file is built as a go/ast tree, printed
// with go/printer and formatted with go/format. A construct Go cannot hold is refused: exit 1 and
// a message naming its position, and nothing is written.
//
//	codespec-write-go <out dir>
//	codespec-write-go --describe
//
// --describe prints the writer's codespec-adapter/1 manifest (describe.go).
package main

import (
	"encoding/json"
	"fmt"
	"io"
	"os"
	"path/filepath"
)

// version is the workspace version, as in codespec-read-go.
const version = "0.2.0"

func main() {
	if len(os.Args) == 2 && os.Args[1] == "--describe" {
		fmt.Print(describe())
		return
	}
	if len(os.Args) != 2 {
		fmt.Fprintln(os.Stderr, "usage: codespec-write-go <out dir> | --describe")
		os.Exit(2)
	}
	if err := run(os.Stdin, os.Args[1]); err != nil {
		fmt.Fprintln(os.Stderr, "codespec-write-go:", err)
		os.Exit(1)
	}
}

func run(in io.Reader, out string) error {
	var m Module
	if err := json.NewDecoder(in).Decode(&m); err != nil {
		return fmt.Errorf("not a cir/1 module: %w", err)
	}
	if m.Format != "cir/1" {
		return fmt.Errorf("format is %q, this writer writes from \"cir/1\"", m.Format)
	}
	src, err := write(&m)
	if err != nil {
		return err
	}
	if err := os.MkdirAll(out, 0o755); err != nil {
		return err
	}
	return os.WriteFile(filepath.Join(out, m.Name+".go"), src, 0o644)
}
