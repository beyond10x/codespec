// Package netcheck checks whether TCP services answer "ping" with "pong".
package netcheck

import (
	"bufio"
	"io"
	"net"
	"strings"
)

// request is the line a probe sends.
var request = "ping\n"

// Prober tells whether the service at addr is up.
type Prober interface {
	Probe(addr string) bool
}

// TCPProber dials the service and sends "ping".
type TCPProber struct{}

// TCP is a Prober that dials.
func TCP() Prober {
	return TCPProber{}
}

// Probe dials addr, sends "ping" and expects "pong".
func (TCPProber) Probe(addr string) bool {
	c, err := net.Dial("tcp", addr)
	if err != nil {
		return false
	}
	_, werr := io.WriteString(c, request)
	if werr != nil {
		c.Close()
		return false
	}
	line, err := bufio.NewReader(c).ReadString('\n')
	c.Close()
	return err == nil && Answer(line) == "pong"
}

// FixedProber answers from a list of addresses that are up. It does no IO.
type FixedProber struct {
	Up []string
}

// Probe reports whether addr is in the list.
func (p FixedProber) Probe(addr string) bool {
	return Count(p.Up, func(a string) bool { return a == addr }) > 0
}

// Answer is the reply to one request line. It only computes.
func Answer(line string) string {
	switch strings.TrimSpace(line) {
	case "ping":
		return "pong"
	case "":
		return "empty"
	default:
		return "unknown"
	}
}

// Count is how many of xs satisfy ok.
func Count[T any](xs []T, ok func(T) bool) int {
	n := 0
	for _, x := range xs {
		if ok(x) {
			n++
		}
	}
	return n
}

// CheckAll probes every address with p and counts those that are up.
func CheckAll(p Prober, addrs ...string) int {
	return Count(addrs, func(a string) bool { return p.Probe(a) })
}
