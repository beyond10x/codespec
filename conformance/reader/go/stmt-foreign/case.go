package stmt_foreign

// Count adds n to a total through a deferred call, which cir/1 cannot express.
func Count(n int) int {
	total := 0
	defer done()
	total += n
	return total
}

func done() {}
