// Package miss connects and reads the next line with the buffered reader the handshake returns.
package miss

import (
	"bufio"
	"net"
)

// Greeting connects to addr and returns the line after the handshake.
func Greeting(addr string) (string, error) {
	conn, br, err := handshake(addr)
	if err != nil {
		return "", err
	}
	defer conn.Close()
	return br.ReadString('\n')
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
