// Package dep masks frames with a mask from math/rand.
package dep

import (
	"encoding/binary"
	"math/rand"
)

// Frame is a payload and its mask.
type Frame struct {
	Mask    [4]byte
	Payload []byte
}

// NewMask returns a random mask.
func NewMask() [4]byte {
	var m [4]byte
	binary.BigEndian.PutUint32(m[:], rand.Uint32())
	return m
}

// MaskFrame gives f a mask.
func MaskFrame(f Frame) Frame {
	f.Mask = NewMask()
	return f
}
