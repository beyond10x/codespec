// Command driver runs a codespec harness trace against the Go output of the ledger example and
// prints one line per command, exactly as the Java and Rust drivers print it.
//
// The implementation is the package at ./ledger, which does not exist on disk: `go build
// -overlay overlay-go.json` supplies it from examples/ledger/out/go. The trace and output formats
// are defined in the doc of crates/harness.
package main

import (
	"bufio"
	"fmt"
	"os"
	"strconv"
	"strings"

	"codespec.example/ledgerdriver/ledger"
)

func main() {
	in := bufio.NewScanner(os.Stdin)
	in.Buffer(make([]byte, 1<<20), 1<<20)
	out := bufio.NewWriter(os.Stdout)
	defer out.Flush()

	var l *ledger.Ledger
	line := 0
	for in.Scan() {
		line++
		f := strings.Split(in.Text(), " ")
		if f[0] == "new" && len(f) == 1 {
			l = ledger.NewLedger()
			fmt.Fprintln(out, "new")
			continue
		}
		if l == nil {
			fail(line, "command before new")
		}
		parse := func(s string, bits int) int64 {
			v, err := strconv.ParseInt(s, 10, bits)
			if err != nil {
				fail(line, err.Error())
			}
			return v
		}
		switch cmd := fmt.Sprintf("%s/%d", f[0], len(f)); cmd {
		case "open/2":
			n, err := l.Open(f[1])
			result(out, "open", err, fmt.Sprintf(" %d", n))
		case "post/4":
			err := l.Post(f[1], kind(line, f[2]), parse(f[3], 64))
			result(out, "post", err, "")
		case "award/3":
			n, err := l.Award(f[1], int32(parse(f[2], 32)))
			result(out, "award", err, fmt.Sprintf(" %d", n))
		case "find/2":
			if a, ok := l.Find(f[1]); ok {
				fmt.Fprintf(out, "find found %s\n", a.Describe())
			} else {
				fmt.Fprintln(out, "find missing")
			}
		case "largest/2":
			if a, ok := l.Find(f[1]); ok {
				fmt.Fprintf(out, "largest %d\n", a.Largest())
			} else {
				fmt.Fprintln(out, "largest missing")
			}
		case "reach/3":
			target := parse(f[2], 64)
			if a, ok := l.Find(f[1]); ok {
				fmt.Fprintf(out, "reach %d\n", a.Reach(target))
			} else {
				fmt.Fprintln(out, "reach missing")
			}
		case "balances/1":
			fmt.Fprintf(out, "balances %s\n", l.Balances())
		case "tallied/1":
			fmt.Fprintf(out, "tallied %d\n", l.Tallied())
		case "count/1":
			fmt.Fprintf(out, "count %d\n", l.Count())
		default:
			fail(line, fmt.Sprintf("unknown command %q", in.Text()))
		}
	}
	if err := in.Err(); err != nil {
		fail(line, err.Error())
	}
}

func kind(line int, s string) ledger.Kind {
	switch s {
	case "deposit":
		return ledger.DEPOSIT
	case "withdrawal":
		return ledger.WITHDRAWAL
	case "fee":
		return ledger.FEE
	}
	fail(line, "unknown kind "+s)
	return ""
}

func result(out *bufio.Writer, cmd string, err error, ok string) {
	if err != nil {
		fmt.Fprintf(out, "%s err %s\n", cmd, err.Error())
	} else {
		fmt.Fprintf(out, "%s ok%s\n", cmd, ok)
	}
}

func fail(line int, msg string) {
	fmt.Fprintf(os.Stderr, "driver: trace line %d: %s\n", line, msg)
	os.Exit(2)
}
