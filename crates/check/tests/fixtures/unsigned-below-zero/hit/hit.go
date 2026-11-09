// Package hit decrements an unsigned hop count, then checks it with <= 0: at zero the
// decrement wraps to the largest value and the check never fires.
package hit

import "errors"

// Hops is a hop count.
type Hops uint32

// Dec takes one hop.
func (h *Hops) Dec() {
	*h = Hops(uint32(*h) - 1)
}

// Val is the count.
func (h Hops) Val() uint32 {
	return uint32(h)
}

// Forward takes a hop and refuses a message that has none left.
func Forward(h *Hops) error {
	h.Dec()
	if h.Val() <= 0 {
		return errors.New("too many hops")
	}
	return nil
}
