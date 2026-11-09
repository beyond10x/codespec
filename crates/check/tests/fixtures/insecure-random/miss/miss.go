// Package miss makes session ids from crypto/rand and uses math/rand only to shuffle.
package miss

import (
	"crypto/rand"
	"encoding/hex"
	mrand "math/rand"
)

// NewSessionID returns the id of a new session.
func NewSessionID() (string, error) {
	b := make([]byte, 16)
	if _, err := rand.Read(b); err != nil {
		return "", err
	}
	return hex.EncodeToString(b), nil
}

// Shuffle puts xs in a random order.
func Shuffle(xs []int) {
	mrand.Shuffle(len(xs), func(i, j int) {
		xs[i], xs[j] = xs[j], xs[i]
	})
}
