// Package advfaults holds what the adversary's fault cases call: errors kept, wrapped in a type,
// a run that stops with an error live, writes undone, and a value filled in after it was stored.
package advfaults

import (
	"os"
	"strconv"
)

// Box keeps the last parse error.
type Box struct {
	Last error
	N    int
}

// NewBox returns an empty box.
func NewBox() *Box { return &Box{} }

// Keep parses s and keeps its error in the box for a later call.
func (b *Box) Keep(s string) {
	n, err := strconv.Atoi(s)
	b.N = n
	b.Last = err
}

// Err returns the kept error.
func (b *Box) Err() error { return b.Last }

// OpError wraps an error with the operation that gave it.
type OpError struct {
	Op  string
	Err error
}

func (e OpError) Error() string { return e.Op + ": " + e.Err.Error() }

// Wrap parses s and returns its error wrapped in an OpError.
func Wrap(s string) error {
	_, err := strconv.Atoi(s)
	if err != nil {
		return OpError{Op: "atoi", Err: err}
	}
	return nil
}

// Late parses s, asks for the process id, then tests the parse error.
func Late(s string) int {
	n, err := strconv.Atoi(s)
	os.Getpid()
	if err != nil {
		return -1
	}
	return n
}

// Gate marks itself busy around a parse and clears the mark before returning.
type Gate struct{ Busy bool }

// NewGate returns an idle gate.
func NewGate() *Gate { return &Gate{} }

// Try parses s while busy.
func (g *Gate) Try(s string) error {
	g.Busy = true
	_, err := strconv.Atoi(s)
	g.Busy = false
	return err
}

// Item is one entry of a Store.
type Item struct {
	Name  string
	State string
}

// Store holds items and announces each one added.
type Store struct {
	items map[string]*Item
	out   chan<- string
}

// NewStore returns an empty store announcing on out.
func NewStore(out chan<- string) *Store {
	return &Store{items: map[string]*Item{}, out: out}
}

// Add stores a new item, then fills in its state, then announces it.
func (s *Store) Add(name string) {
	it := &Item{Name: name}
	s.items[name] = it
	it.State = "added"
	s.out <- name
}

// Get returns the item named name.
func (s *Store) Get(name string) Item { return *s.items[name] }
