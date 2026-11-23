// Package reused dials a connection and reuses the dial's error variable for a second call: the
// second call's error branch in Send returns with the connection still open; Fresh closes it
// there.
package reused

import (
	"net"
	"time"
)

// Send connects to addr and sets its write deadline.
func Send(addr string, by time.Time) (net.Conn, error) {
	conn, err := net.Dial("tcp", addr)
	if err != nil {
		return nil, err
	}
	err = conn.SetWriteDeadline(by)
	if err != nil {
		return nil, err
	}
	return conn, nil
}

// Fresh connects to addr and sets its write deadline, closing the connection when that fails.
func Fresh(addr string, by time.Time) (net.Conn, error) {
	conn, err := net.Dial("tcp", addr)
	if err != nil {
		return nil, err
	}
	err = conn.SetWriteDeadline(by)
	if err != nil {
		conn.Close()
		return nil, err
	}
	return conn, nil
}
