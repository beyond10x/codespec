// Package miss records the sender of a datagram while it handles it, and forgets it after; it
// also returns buffers read from a plain reader to a pool, which is no container.
package miss

import (
	"io"
	"net"
	"sync"
)

// Peers is keyed by the sender's address.
type Peers struct {
	m map[string]net.Addr
}

// Add records a peer.
func (p *Peers) Add(a net.Addr) {
	p.m[a.String()] = a
}

// Delete forgets a peer.
func (p *Peers) Delete(a net.Addr) {
	delete(p.m, a.String())
}

// Serve records the sender of every datagram while it handles it.
func Serve(c net.PacketConn, p *Peers) error {
	buf := make([]byte, 1500)
	for {
		n, addr, err := c.ReadFrom(buf)
		if err != nil {
			return err
		}
		p.Add(addr)
		handle(buf[:n])
		forget(p, addr)
	}
}

func handle(b []byte) {}

func forget(p *Peers, a net.Addr) {
	p.Delete(a)
}

var pool sync.Pool

// Fill reads from r into a buffer and returns the buffer to the pool.
func Fill(r io.Reader, b *[]byte) error {
	_, err := r.Read(*b)
	pool.Put(b)
	return err
}
