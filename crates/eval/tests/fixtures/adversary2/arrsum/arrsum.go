// Package arrsum adds the elements of an array of 2^20 ints by index.
package arrsum

// Sum adds the elements of an array of 2^20 ints.
func Sum() int {
	var a [1048576]int
	s := 0
	for i := 0; i < len(a); i++ {
		s += a[i]
	}
	return s
}
