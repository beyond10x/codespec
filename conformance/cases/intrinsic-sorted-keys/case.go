package intrinsic_sorted_keys

import "sort"

// case.cir.json is written by hand: Go has no expression for a map's keys in order, and the Go
// reader reads none of the loops and sort calls below as one. The Java reader writes sorted_keys
// for a loop over a TreeMap's keySet.

// Join concatenates the distinct strings in ascending order.
func Join(a string, b string, c string) string {
	m := map[string]int{}
	m[a] = 1
	m[b] = 2
	m[c] = 3
	keys := make([]string, 0, len(m))
	for k := range m {
		keys = append(keys, k)
	}
	sort.Strings(keys)
	s := ""
	for _, k := range keys {
		s += k
	}
	return s
}

// Ints gives the distinct integers in ascending order.
func Ints(a int, b int, c int) []int {
	m := map[int]int{}
	m[a] = a
	m[b] = b
	m[c] = c
	keys := make([]int, 0, len(m))
	for k := range m {
		keys = append(keys, k)
	}
	sort.Ints(keys)
	return keys
}
