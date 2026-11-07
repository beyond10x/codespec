// Command driver runs a codespec harness trace against the Go output of the fees example and
// prints one line per command, exactly as the Groovy and Rust drivers print it.
//
// The implementation is the package at ./fees, which does not exist on disk: `go build -overlay
// overlay-go.json` supplies it from examples/fees/out/go. The trace and output formats are defined
// in the doc of crates/harness.
package main

import (
	"bufio"
	"fmt"
	"os"
	"strconv"
	"strings"

	"codespec.example/feesdriver/fees"
)

func main() {
	in := bufio.NewScanner(os.Stdin)
	in.Buffer(make([]byte, 1<<20), 1<<20)
	out := bufio.NewWriter(os.Stdout)
	defer out.Flush()

	var s *fees.LateFeeService
	line := 0
	for in.Scan() {
		line++
		f := strings.Split(in.Text(), " ")
		if f[0] == "new" && len(f) == 1 {
			s = fees.NewLateFeeService()
			fmt.Fprintln(out, "new")
			continue
		}
		if s == nil {
			fail(line, "command before new")
		}
		parse := func(v string, bits int) int64 {
			n, err := strconv.ParseInt(v, 10, bits)
			if err != nil {
				fail(line, err.Error())
			}
			return n
		}
		switch cmd := fmt.Sprintf("%s/%d", f[0], len(f)); cmd {
		case "rate/3":
			fmt.Fprintf(out, "rate %d\n", s.PutRate(kind(line, f[1]), parse(f[2], 64)))
		case "grace/2":
			fmt.Fprintf(out, "grace %d\n", s.GraceFor(int32(parse(f[1], 32))))
		case "cap/2":
			fmt.Fprintf(out, "cap %d\n", s.CapAt(parse(f[1], 64)))
		case "waive/2":
			fmt.Fprintf(out, "waive %d\n", s.Waive(int32(parse(f[1], 32))))
		case "assess/3":
			fmt.Fprintf(out, "assess %d\n", s.Assess(kind(line, f[1]), int32(parse(f[2], 32))))
		case "rates/1":
			fmt.Fprintf(out, "rates %s\n", s.RateList())
		case "stats/1":
			fmt.Fprintf(out, "stats %s\n", s.Stats())
		default:
			fail(line, fmt.Sprintf("unknown command %q", in.Text()))
		}
	}
	if err := in.Err(); err != nil {
		fail(line, err.Error())
	}
}

func kind(line int, s string) fees.ItemKind {
	switch s {
	case "book":
		return fees.BOOK
	case "disc":
		return fees.DISC
	case "tool":
		return fees.TOOL
	}
	fail(line, "unknown kind "+s)
	return ""
}

func fail(line int, msg string) {
	fmt.Fprintf(os.Stderr, "driver: trace line %d: %s\n", line, msg)
	os.Exit(2)
}
