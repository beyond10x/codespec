// Package tally counts items.
package tally

import "strings"

// Item is one counted thing.
type Item struct {
	Name  string
	Count int
}

// Add returns the item with n more.
func (it Item) Add(n int) Item {
	return Item{Name: it.Name, Count: it.Count + n}
}

func total(items []Item) int {
	sum := 0
	for _, it := range items {
		sum += it.Count
	}
	return sum
}

// Total sums the items whose name has the prefix — each match once.
func Total(items []Item, prefix string) int {
	var picked []Item
	for _, it := range items {
		if strings.HasPrefix(it.Name, prefix) {
			picked = append(picked, it.Add(0))
		}
	}
	return total(picked)
}
