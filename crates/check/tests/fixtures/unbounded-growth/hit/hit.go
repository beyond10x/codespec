// Package hit remembers every peer that sends a datagram and never forgets one; it receives
// through its own connection type.
package hit

import "net"

// Conn is a packet connection.
type Conn struct {
	pc net.PacketConn
}

// ReadFrom reads one datagram.
func (c *Conn) ReadFrom(b []byte) (int, net.Addr, error) {
	return c.pc.ReadFrom(b)
}

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

// Serve records the sender of every datagram it reads.
func Serve(c *Conn, p *Peers) error {
	buf := make([]byte, 1500)
	for {
		_, addr, err := c.ReadFrom(buf)
		if err != nil {
			return err
		}
		p.Add(addr)
	}
}
