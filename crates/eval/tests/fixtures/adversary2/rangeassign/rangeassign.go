// Package rangeassign ranges with = into a variable declared before the loop.
package rangeassign

// Last gives the last index a range over three elements assigns to i.
func Last() int {
	xs := []int{}
	xs = append(xs, 5, 6, 7)
	var i int
	for i = range xs {
	}
	return i
}
