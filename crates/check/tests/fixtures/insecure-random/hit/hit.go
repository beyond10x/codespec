// Package hit makes session ids from math/rand.
package hit

import (
	"math/rand"
	"strconv"
)

// NewSessionID returns the id of a new session.
func NewSessionID() string {
	return strconv.Itoa(digits())
}

func digits() int {
	return rand.Intn(1000000)
}
