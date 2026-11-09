// Package hit dials a connection, wraps it in TLS, and returns an error for a missing server
// name without closing it.
package hit

import (
	"crypto/tls"
	"errors"
	"net"
)

// Dial connects to addr over TLS.
func Dial(addr string, name string, cfg *tls.Config) (*tls.Conn, error) {
	conn, err := net.Dial("tcp", addr)
	if err != nil {
		return nil, err
	}
	tc := tls.Client(conn, cfg)
	if name == "" {
		return nil, errors.New("no server name")
	}
	err = tc.Handshake()
	if err != nil {
		tc.Close()
		return nil, err
	}
	return tc, nil
}
