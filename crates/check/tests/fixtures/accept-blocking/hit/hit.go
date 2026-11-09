// Package hit reads each peer's greeting in the accept loop itself, with no deadline: one peer
// that connects and sends nothing stops every later accept.
package hit

import (
	"io"
	"net"
)

// Serve accepts peers, reads a four-byte greeting from each, and hands it on.
func Serve(l net.Listener, handle func(net.Conn, []byte)) error {
	for {
		conn, err := l.Accept()
		if err != nil {
			return err
		}
		greeting := make([]byte, 4)
		_, err = io.ReadFull(conn, greeting)
		if err != nil {
			conn.Close()
			continue
		}
		go handle(conn, greeting)
	}
}
