// Package constfold computes with constant expressions, which Go evaluates exactly at compile
// time; the Go reader keeps them as binops of their operands.
package constfold

import "fmt"

const big = 1 << 62

// Scaled is big * 4 / 8, exact in Go: 1 << 61.
func Scaled() int {
	return big * 4 / 8
}

// Tenths prints 0.1 + 0.2, exact in Go: 0.3.
func Tenths() {
	f := 0.1 + 0.2
	fmt.Println(f)
}
