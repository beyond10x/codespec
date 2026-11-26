// Package conn closes its connection in a deferred function literal: no path returns with it
// open.
package conn

import (
	"errors"
	"net"
)

// Ping connects to addr, writes one byte, and closes the connection when it returns.
func Ping(addr string) error {
	conn, err := net.Dial("tcp", addr)
	if err != nil {
		return err
	}
	defer func() {
		_ = conn.Close()
	}()
	_, err = conn.Write([]byte{0})
	if err != nil {
		return errors.New("ping failed")
	}
	return nil
}
