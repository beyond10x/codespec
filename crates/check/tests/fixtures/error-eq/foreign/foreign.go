// Package foreign compares a context's cause with == inside an if with an init statement that cir/1
// keeps foreign, its name being bound again after it; a sibling matches it with errors.Is the same way.
package foreign

import (
	"context"
	"errors"
)

// ErrForce cancels a wait at once.
var ErrForce = errors.New("force cancel")

// Wait returns at once when ctx was cancelled with ErrForce, or else with what done returns.
func Wait(ctx context.Context, done func() error) error {
	if err := context.Cause(ctx); err == ErrForce {
		return ctx.Err()
	}
	err := done()
	return err
}

// WaitIs returns at once when ctx was cancelled with ErrForce, or else with what done returns.
func WaitIs(ctx context.Context, done func() error) error {
	if err := context.Cause(ctx); errors.Is(err, ErrForce) {
		return ctx.Err()
	}
	err := done()
	return err
}
