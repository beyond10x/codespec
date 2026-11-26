package probe

import (
	"context"
	"log/slog"
	"time"
)

func Run(parent context.Context) time.Time {
	ctx, cancel := context.WithTimeout(parent, time.Second)
	defer cancel()
	_ = slog.String("k", "v")
	_ = ctx
	return time.Now()
}
