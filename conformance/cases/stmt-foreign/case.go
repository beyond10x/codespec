package stmt_foreign

// Count adds n to a total and starts a goroutine, which cir/1 cannot express.
func Count(n int) int {
	total := 0
	go done()
	total += n
	return total
}

func done() {}
