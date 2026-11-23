// Package reaching compares an error local with == where only the definition that reaches the
// comparison decides: one function holds a cause there and assigns a plain error after it, one
// the other way round, one assigns the cause on one branch, one on an earlier turn of a loop.
package reaching

import (
	"context"
	"errors"
)

// ErrForce cancels a wait at once.
var ErrForce = errors.New("force cancel")

// ErrPlain is the error plain returns.
var ErrPlain = errors.New("plain")

func plain() error {
	return ErrPlain
}

// CauseFirst compares the cause, then assigns a plain error.
func CauseFirst(ctx context.Context) bool {
	err := context.Cause(ctx)
	if err == ErrForce {
		return true
	}
	err = plain()
	return err != nil
}

// PlainFirst compares a plain error, then assigns the cause.
func PlainFirst(ctx context.Context) bool {
	err := plain()
	if err == ErrPlain {
		return true
	}
	err = context.Cause(ctx)
	return err != nil
}

// MaybeCause assigns the cause on one branch only.
func MaybeCause(ctx context.Context, late bool) bool {
	err := plain()
	if late {
		err = context.Cause(ctx)
	}
	return err == ErrForce
}

// InLoop compares on each turn what the turn before assigned.
func InLoop(ctx context.Context, n int) int {
	err := plain()
	for i := 0; i < n; i++ {
		if err == ErrForce {
			return i
		}
		err = context.Cause(ctx)
	}
	return n
}
