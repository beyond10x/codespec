// Command fromrust runs a codespec harness trace against the series package written as Go from
// the Rust output, and prints one line per command.
//
// The implementation is the package at ../series, which does not exist on disk: `go build
// -overlay overlay-go-from-rust.json ./fromrust` supplies it from examples/series/out/go-from-rust.
// That package is read from Rust, so its integers are int64 and New returns a Series value;
// otherwise this is the driver in ../main.go. The trace and output formats are defined in the doc
// of crates/harness.
package main

import (
	"bufio"
	"fmt"
	"os"
	"strconv"
	"strings"

	"codespec.example/seriesdriver/series"
)

func main() {
	in := bufio.NewScanner(os.Stdin)
	in.Buffer(make([]byte, 1<<20), 1<<20)
	out := bufio.NewWriter(os.Stdout)
	defer out.Flush()

	var s *series.Series
	line := 0
	for in.Scan() {
		line++
		f := strings.Split(in.Text(), " ")
		if f[0] != "new" && s == nil {
			fail(line, "command before new")
		}
		arg := func() int64 {
			n, err := strconv.ParseInt(f[1], 10, 64)
			if err != nil {
				fail(line, err.Error())
			}
			return n
		}
		switch {
		case f[0] == "new" && len(f) == 1:
			v := series.New()
			s = &v
			fmt.Fprintln(out, "new")
		case f[0] == "push" && len(f) == 2:
			fmt.Fprintf(out, "push %d\n", s.Push(arg()))
		case f[0] == "sum" && len(f) == 1:
			fmt.Fprintf(out, "sum %d\n", s.Sum())
		case f[0] == "mean" && len(f) == 1:
			v, err := s.Mean()
			result(out, "mean", v, err)
		case f[0] == "median" && len(f) == 1:
			v, err := s.Median()
			result(out, "median", v, err)
		case f[0] == "sorted" && len(f) == 1:
			vs := s.Sorted()
			fmt.Fprintf(out, "sorted %d", len(vs))
			for _, v := range vs {
				fmt.Fprintf(out, " %d", v)
			}
			fmt.Fprintln(out)
		case f[0] == "reverse" && len(f) == 1:
			s.Reverse()
			fmt.Fprintln(out, "reverse")
		case f[0] == "trend" && len(f) == 1:
			fmt.Fprintf(out, "trend %s\n", s.Trend())
		case f[0] == "window" && len(f) == 2:
			v, err := s.MaxWindow(arg())
			result(out, "window", v, err)
		case f[0] == "gcd" && len(f) == 1:
			fmt.Fprintf(out, "gcd %d\n", s.Gcd())
		case f[0] == "evens" && len(f) == 1:
			fmt.Fprintf(out, "evens %d\n", s.EvenSum())
		case f[0] == "drop" && len(f) == 2:
			fmt.Fprintf(out, "drop %d\n", s.Drop(arg()))
		case f[0] == "len" && len(f) == 1:
			fmt.Fprintf(out, "len %d\n", s.Len())
		case f[0] == "steps" && len(f) == 2:
			v, err := series.Steps(arg())
			result(out, "steps", v, err)
		case f[0] == "prime" && len(f) == 2:
			fmt.Fprintf(out, "prime %t\n", series.IsPrime(arg()))
		case f[0] == "bucket" && len(f) == 2:
			fmt.Fprintf(out, "bucket %s\n", series.Bucket(arg()))
		case f[0] == "residue" && len(f) == 2:
			fmt.Fprintf(out, "residue %s\n", series.Residue(arg()))
		default:
			fail(line, fmt.Sprintf("unknown command %q", in.Text()))
		}
	}
	if err := in.Err(); err != nil {
		fail(line, err.Error())
	}
}

func result(out *bufio.Writer, cmd string, v int64, err error) {
	if err != nil {
		fmt.Fprintf(out, "%s err %s\n", cmd, err.Error())
	} else {
		fmt.Fprintf(out, "%s ok %d\n", cmd, v)
	}
}

func fail(line int, msg string) {
	fmt.Fprintf(os.Stderr, "driver: trace line %d: %s\n", line, msg)
	os.Exit(2)
}
