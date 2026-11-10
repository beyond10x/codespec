// Package casesites has two functions whose names differ only in letter case, each with one
// call of strconv.Atoi: two fault sites whose ids differ only in case.
package casesites

import "strconv"

// Parse gives the number s spells.
func Parse(s string) int {
	n, _ := strconv.Atoi(s)
	return n + parse(s)
}

func parse(s string) int {
	n, _ := strconv.Atoi(s)
	return n
}
