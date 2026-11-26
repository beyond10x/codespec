package stmt_defer

import "fmt"

// Count prints in the order its deferred calls run: the last deferred first, each with the
// argument it had when it was deferred, and after the result is taken, so the literal that
// clears n does not change what Count returns.
func Count(n int) int {
	defer show("first", n)
	n++
	defer show("second", n)
	defer func() {
		n = 0
	}()
	fmt.Println("body", n)
	return n
}

func show(what string, v int) {
	fmt.Println(what, v)
}
