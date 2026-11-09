// Package hit finds a header by name ignoring case, but removes one only on an exact match.
package hit

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

// GetHeader returns the value of the first header named name, in any case.
func (h *Headers) GetHeader(name string) string {
	for _, x := range h.list {
		if strings.ToLower(x.Name) == strings.ToLower(name) {
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
