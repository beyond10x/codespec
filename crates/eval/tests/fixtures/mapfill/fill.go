// Package fill puts n distinct keys into a map and looks each one up again.
package fill

import "strconv"

// Fill sets m[i] = i for i below n, then sums m[i] over the same keys.
func Fill(n int) int {
	m := map[int]int{}
	for i := 0; i < n; i++ {
		m[i] = i
	}
	sum := 0
	for i := 0; i < n; i++ {
		sum += m[i]
	}
	return sum
}

// Names counts the distinct strings among the keys "0" .. "<n-1>", each written twice.
func Names(n int) int {
	m := map[string]bool{}
	for r := 0; r < 2; r++ {
		for i := 0; i < n; i++ {
			m[strconv.Itoa(i)] = true
		}
	}
	return len(m)
}
