package near

import (
	"context"
	"crypto/rand"
	"errors"
	"fmt"
	"io"
	"log"
	"log/slog"
	"net/http"
)

func Info(l *slog.Logger) {
	l.Info("started")
}

func Fatal() {
	log.Fatal("stopped")
}

func Background() context.Context {
	return context.Background()
}

func NewError() error {
	return errors.New("failed")
}

func Get() (*http.Response, error) {
	return http.Get("https://example.com/")
}

func Write(w io.Writer) (int, error) {
	return fmt.Fprintf(w, "%d\n", 1)
}

func Random(b []byte) (int, error) {
	return rand.Read(b)
}
