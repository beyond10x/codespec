// Package boundary sends frames masked by a dependency that takes the mask from math/rand.
package boundary

import "example.com/checks/insecure-random/dep"

// Send gives p a frame with a mask.
func Send(p []byte) dep.Frame {
	return dep.MaskFrame(dep.Frame{Payload: p})
}

// Plain gives p a frame with no mask.
func Plain(p []byte) dep.Frame {
	return dep.Frame{Payload: p}
}
