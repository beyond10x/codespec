// Package miss matches a context's cause with errors.Is, and compares with == only an error
// its own function returns unwrapped.
package miss

import (
	"context"
	"errors"
)

// ErrForce cancels a wait at once.
var ErrForce = errors.New("force cancel")

// ErrMissing is the error of a lookup that finds nothing.
var ErrMissing = errors.New("missing")

// Wait returns at once when ctx was cancelled with ErrForce, or else with what done returns.
func Wait(ctx context.Context, done func() error) error {
	if errors.Is(context.Cause(ctx), ErrForce) {
		return ctx.Err()
	}
	return done()
}

func find(m map[string]int, k string) (int, error) {
	v, ok := m[k]
	if !ok {
		return 0, ErrMissing
	}
	return v, nil
}

// Has reports whether m holds k.
func Has(m map[string]int, k string) bool {
	_, err := find(m, k)
	return err != ErrMissing
}
