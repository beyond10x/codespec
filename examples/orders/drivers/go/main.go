// Command driver runs a codespec harness trace against one Go implementation of the orders
// package and prints one line per command result, then one line per event the call sent.
//
// The implementation is the package at ./orders, which does not exist on disk: `go build
// -overlay overlay-original.json` supplies it from examples/orders/source, `-overlay
// overlay-go.json` from examples/orders/out/go. The trace and output formats are defined in the
// doc of crates/harness.
package main

import (
	"bufio"
	"fmt"
	"os"
	"sort"
	"strconv"
	"strings"

	"codespec.example/ordersdriver/orders"
)

func main() {
	in := bufio.NewScanner(os.Stdin)
	in.Buffer(make([]byte, 1<<20), 1<<20)
	out := bufio.NewWriter(os.Stdout)
	defer out.Flush()

	var svc *orders.Service
	var events chan any
	line := 0
	for in.Scan() {
		line++
		f := strings.Split(in.Text(), " ")
		if f[0] != "new" && svc == nil {
			fail(line, "command before new")
		}
		switch {
		case f[0] == "new" && len(f) == 1:
			events = make(chan any, 16)
			svc = orders.New(events)
			fmt.Fprintln(out, "new")
		case f[0] == "place" && len(f) == 4:
			total, err := strconv.ParseInt(f[2], 10, 64)
			if err != nil {
				fail(line, err.Error())
			}
			gift, err := strconv.ParseBool(f[3])
			if err != nil {
				fail(line, err.Error())
			}
			id, err := svc.Place(f[1], total, gift)
			if err != nil {
				fmt.Fprintf(out, "place err %s\n", err.Error())
			} else {
				fmt.Fprintf(out, "place ok %s\n", id)
			}
		case f[0] == "pay" && len(f) == 2:
			result(out, "pay", svc.Pay(orders.OrderID(f[1])))
		case f[0] == "ship" && len(f) == 3:
			result(out, "ship", svc.Ship(orders.OrderID(f[1]), f[2]))
		case f[0] == "cancel" && len(f) == 2:
			result(out, "cancel", svc.Cancel(orders.OrderID(f[1])))
		case f[0] == "get" && len(f) == 2:
			o, ok := svc.Get(orders.OrderID(f[1]))
			if ok {
				fmt.Fprintf(out, "get found %s\n", order(o))
			} else {
				fmt.Fprintln(out, "get missing")
			}
		case f[0] == "pending" && len(f) == 1:
			ps := svc.PendingOrders()
			sort.Slice(ps, func(i, j int) bool { return ps[i].ID < ps[j].ID })
			fmt.Fprintf(out, "pending %d", len(ps))
			for _, o := range ps {
				fmt.Fprintf(out, " | %s", order(o))
			}
			fmt.Fprintln(out)
		default:
			fail(line, fmt.Sprintf("unknown command %q", in.Text()))
		}
		drain(out, events)
	}
	if err := in.Err(); err != nil {
		fail(line, err.Error())
	}
}

func result(out *bufio.Writer, cmd string, err error) {
	if err != nil {
		fmt.Fprintf(out, "%s err %s\n", cmd, err.Error())
	} else {
		fmt.Fprintf(out, "%s ok\n", cmd)
	}
}

func order(o orders.Order) string {
	return fmt.Sprintf("id=%s customer=%s total=%d status=%s tracking=%s gift=%t",
		o.ID, o.Customer, o.Total, o.Status, o.Tracking, o.Gift)
}

func drain(out *bufio.Writer, events chan any) {
	for {
		select {
		case e := <-events:
			switch e := e.(type) {
			case orders.OrderPlaced:
				fmt.Fprintf(out, "event OrderPlaced id=%s customer=%s total=%d\n", e.ID, e.Customer, e.Total)
			case orders.OrderPaid:
				fmt.Fprintf(out, "event OrderPaid id=%s\n", e.ID)
			case orders.OrderShipped:
				fmt.Fprintf(out, "event OrderShipped id=%s tracking=%s\n", e.ID, e.Tracking)
			case orders.OrderCancelled:
				fmt.Fprintf(out, "event OrderCancelled id=%s\n", e.ID)
			default:
				fmt.Fprintf(out, "event unknown %T\n", e)
			}
		default:
			return
		}
	}
}

func fail(line int, msg string) {
	fmt.Fprintf(os.Stderr, "driver: trace line %d: %s\n", line, msg)
	os.Exit(2)
}
