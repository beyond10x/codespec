// Package wrapper folds case with functions of its own, one that lowers ASCII by hand and one
// that calls strings.ToLower, and removes entries only on an exact match.
package wrapper

import "strings"

// Header is one name and value.
type Header struct {
	Name  string
	Value string
}

// Headers is a message's header list.
type Headers struct {
	list []Header
}

// lowerName lowers the ASCII letters of s.
func lowerName(s string) string {
	b := []byte(s)
	for i, c := range b {
		if 'A' <= c && c <= 'Z' {
			b[i] = c + 'a' - 'A'
		}
	}
	return string(b)
}

// GetHeader returns the value of the first header named name, in any case.
func (h *Headers) GetHeader(name string) string {
	for _, x := range h.list {
		if lowerName(x.Name) == lowerName(name) {
			return x.Value
		}
	}
	return ""
}

// RemoveHeader removes the first header named name.
func (h *Headers) RemoveHeader(name string) bool {
	for i, x := range h.list {
		if x.Name == name {
			h.list = append(h.list[:i], h.list[i+1:]...)
			return true
		}
	}
	return false
}

// Param is one key and value.
type Param struct {
	Key   string
	Value string
}

// Params is a parameter list.
type Params struct {
	list []Param
}

func norm(s string) string {
	return strings.ToLower(s)
}

// GetParam returns the value of the first parameter keyed key, in any case.
func (p *Params) GetParam(key string) string {
	for _, x := range p.list {
		if norm(x.Key) == norm(key) {
			return x.Value
		}
	}
	return ""
}

// RemoveParam removes the first parameter keyed key.
func (p *Params) RemoveParam(key string) bool {
	for i, x := range p.list {
		if x.Key == key {
			p.list = append(p.list[:i], p.list[i+1:]...)
			return true
		}
	}
	return false
}
