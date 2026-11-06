// Package greeter greets people by name and counts how often each was greeted.
package greeter

import (
	"errors"
	"fmt"
)

var ErrNoName = errors.New("name must not be empty")

type Greeter struct {
	greeted map[string]int
}

func New() *Greeter {
	return &Greeter{greeted: map[string]int{}}
}

func (g *Greeter) Greet(name string) (string, error) {
	if name == "" {
		return "", ErrNoName
	}
	g.greeted[name] = g.greeted[name] + 1
	return fmt.Sprintf("Hello, %s!", name), nil
}

func (g *Greeter) Count(name string) (int, bool) {
	n, ok := g.greeted[name]
	if !ok {
		return 0, false
	}
	return n, true
}
