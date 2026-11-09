// Package hit compares a context's cause with == to a sentinel: a cause that wraps the
// sentinel does not match.
package hit

import (
	"context"
	"errors"
)

// ErrForce cancels a wait at once.
var ErrForce = errors.New("force cancel")

// Wait returns at once when ctx was cancelled with ErrForce, or else with what done returns.
func Wait(ctx context.Context, done func() error) error {
	if context.Cause(ctx) == ErrForce {
		return ctx.Err()
	}
	return done()
}
