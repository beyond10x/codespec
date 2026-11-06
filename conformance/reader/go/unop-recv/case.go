package unop_recv

// Echo sends n and receives it back.
func Echo(ch chan int, n int) int {
	ch <- n
	return <-ch
}
