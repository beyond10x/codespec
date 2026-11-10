// Package nilrecv calls a pointer-receiver method through an interface holding a nil pointer.
package nilrecv

// T is a struct.
type T struct{ v int }

// Name handles a nil receiver.
func (t *T) Name() string {
	if t == nil {
		return "nil"
	}
	return "t"
}

// Namer names.
type Namer interface{ Name() string }

// NilRecv calls Name through a Namer holding a nil *T.
func NilRecv() string {
	var p *T
	var n Namer
	n = p
	return n.Name()
}
