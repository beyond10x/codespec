// Command driver runs a codespec harness trace against the Go output of the inventory example and
// prints one line per command, exactly as the Rust driver prints it.
//
// The implementation is the package at ./inventory, which does not exist on disk: `go build
// -overlay overlay-go.json` supplies it from examples/inventory/out/go. The trace and output
// formats are defined in the doc of crates/harness.
package main

import (
	"bufio"
	"fmt"
	"os"
	"sort"
	"strconv"
	"strings"

	"codespec.example/inventorydriver/inventory"
)

func main() {
	in := bufio.NewScanner(os.Stdin)
	in.Buffer(make([]byte, 1<<20), 1<<20)
	out := bufio.NewWriter(os.Stdout)
	defer out.Flush()

	var inv *inventory.Inventory
	line := 0
	for in.Scan() {
		line++
		f := strings.Split(in.Text(), " ")
		if f[0] == "new" && len(f) == 1 {
			v := inventory.New()
			inv = &v
			fmt.Fprintln(out, "new")
			continue
		}
		if inv == nil {
			fail(line, "command before new")
		}
		switch {
		case f[0] == "add" && len(f) == 5:
			if err := inv.AddItem(f[1], f[2], category(line, f[3]), integer(line, f[4])); err != nil {
				fmt.Fprintf(out, "add err %s\n", err.Error())
			} else {
				fmt.Fprintln(out, "add ok")
			}
		case f[0] == "receive" && len(f) == 3:
			v, err := inv.Receive(f[1], integer(line, f[2]))
			level(out, "receive", v, err)
		case f[0] == "reserve" && len(f) == 3:
			v, err := inv.Reserve(f[1], integer(line, f[2]))
			level(out, "reserve", v, err)
		case f[0] == "find" && len(f) == 2:
			item, ok := inv.Find(f[1])
			if ok {
				fmt.Fprintf(out, "find found %s %s\n", item.Sku, item.Label())
			} else {
				fmt.Fprintln(out, "find missing")
			}
		case f[0] == "stock" && len(f) == 2:
			fmt.Fprintf(out, "stock %d\n", inv.StockOf(f[1]))
		case f[0] == "value" && len(f) == 2:
			c := category(line, f[1])
			fmt.Fprintf(out, "value %s %d\n", c, inv.ValueOf(c))
		case f[0] == "low" && len(f) == 2:
			skus := inv.LowStock(integer(line, f[1]))
			sort.Strings(skus)
			fmt.Fprintf(out, "low %d", len(skus))
			for _, s := range skus {
				fmt.Fprintf(out, " | %s", s)
			}
			fmt.Fprintln(out)
		case f[0] == "reserved" && len(f) == 1:
			fmt.Fprintf(out, "reserved %d\n", inv.Reserved())
		default:
			fail(line, fmt.Sprintf("unknown command %q", in.Text()))
		}
	}
	if err := in.Err(); err != nil {
		fail(line, err.Error())
	}
}

func level(out *bufio.Writer, cmd string, v int64, err error) {
	if err != nil {
		fmt.Fprintf(out, "%s err %s\n", cmd, err.Error())
	} else {
		fmt.Fprintf(out, "%s ok %d\n", cmd, v)
	}
}

func integer(line int, s string) int64 {
	v, err := strconv.ParseInt(s, 10, 64)
	if err != nil {
		fail(line, err.Error())
	}
	return v
}

func category(line int, s string) inventory.Category {
	switch s {
	case "tool":
		return inventory.Tool
	case "part":
		return inventory.Part
	case "supply":
		return inventory.Supply
	}
	fail(line, fmt.Sprintf("unknown category %q", s))
	return 0
}

func fail(line int, msg string) {
	fmt.Fprintf(os.Stderr, "driver: trace line %d: %s\n", line, msg)
	os.Exit(2)
}
