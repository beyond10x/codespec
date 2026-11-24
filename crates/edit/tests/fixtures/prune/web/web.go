// Package web serves items.
package web

import "example.com/store"

// Items counts the items twice.
func Items() int {
	n := store.Load()
	return n * 2
}

func stale() int {
	return store.Fetch(0)
}

// Page gives a page.
func Page() int {
	return 1
}
