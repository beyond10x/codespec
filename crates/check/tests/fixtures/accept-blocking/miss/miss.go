// Package miss reads each peer's greeting with a deadline set first, or in the goroutine it
// starts per peer.
package miss

import (
	"io"
	"net"
	"time"
)

// Serve accepts peers, reads a four-byte greeting from each within a second, and hands it on.
func Serve(l net.Listener, handle func(net.Conn, []byte)) error {
	for {
		conn, err := l.Accept()
		if err != nil {
			return err
		}
		conn.SetReadDeadline(time.Now().Add(time.Second))
		greeting := make([]byte, 4)
		_, err = io.ReadFull(conn, greeting)
		if err != nil {
			conn.Close()
			continue
		}
		go handle(conn, greeting)
	}
}

// ServeEach accepts peers and reads each one's greeting in its own goroutine.
func ServeEach(l net.Listener, handle func(net.Conn, []byte)) error {
	for {
		conn, err := l.Accept()
		if err != nil {
			return err
		}
		go greet(conn, handle)
	}
}

func greet(conn net.Conn, handle func(net.Conn, []byte)) {
	greeting := make([]byte, 4)
	if _, err := io.ReadFull(conn, greeting); err != nil {
		conn.Close()
		return
	}
	handle(conn, greeting)
}
