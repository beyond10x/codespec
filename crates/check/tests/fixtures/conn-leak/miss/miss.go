// Package miss closes, returns or hands on every connection it dials, on every path.
package miss

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
		tc.Close()
		return nil, errors.New("no server name")
	}
	err = tc.Handshake()
	if err != nil {
		tc.Close()
		return nil, err
	}
	return tc, nil
}

// Ping connects to addr, writes one byte and closes the connection when it returns.
func Ping(addr string) error {
	conn, err := net.Dial("tcp", addr)
	if err != nil {
		return err
	}
	defer conn.Close()
	_, err = conn.Write([]byte{0})
	if err != nil {
		return errors.New("ping failed")
	}
	return nil
}

// Pool keeps one connection.
type Pool struct {
	conn net.Conn
}

// Open connects to addr and keeps the connection in p.
func (p *Pool) Open(addr string, ok bool) error {
	conn, err := net.Dial("tcp", addr)
	if err != nil {
		return err
	}
	p.conn = conn
	if !ok {
		return errors.New("kept, not ready")
	}
	return nil
}
