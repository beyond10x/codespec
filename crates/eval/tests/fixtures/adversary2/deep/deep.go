// Package deep recurses 990 calls deep through an expression of 25 additions, within the
// evaluator's stated nesting of 1000 calls.
package deep

func f(n int) int {
	if n == 0 {
		return 0
	}
	return f(n-1) + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1
}

// Deep recurses 990 calls deep.
func Deep() int { return f(990) }
