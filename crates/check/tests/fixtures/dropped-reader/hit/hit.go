// Package hit connects and drops the buffered reader the handshake returns, with the bytes it
// already read past the handshake.
package hit

import (
	"bufio"
	"net"
)

// Dial connects to addr.
func Dial(addr string) (net.Conn, error) {
	conn, _, err := handshake(addr)
	return conn, err
}

func handshake(addr string) (net.Conn, *bufio.Reader, error) {
	c, err := net.Dial("tcp", addr)
	if err != nil {
		return nil, nil, err
	}
	br := bufio.NewReader(c)
	_, err = br.ReadString('\n')
	if err != nil {
		c.Close()
		return nil, nil, err
	}
	return c, br, nil
}
