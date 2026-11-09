// Package miss checks an unsigned hop count with == 0 before it takes a hop, and a signed
// budget with <= 0.
package miss

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

// Forward refuses a message that has no hop left, then takes one.
func Forward(h *Hops) error {
	if h.Val() == 0 {
		return errors.New("too many hops")
	}
	h.Dec()
	return nil
}

// Spend takes n from budget and refuses an overdraft.
func Spend(budget int, n int) (int, error) {
	budget -= n
	if budget <= 0 {
		return 0, errors.New("over budget")
	}
	return budget, nil
}
