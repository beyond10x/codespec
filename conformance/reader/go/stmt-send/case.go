package stmt_send

// Emit sends n and n+1.
func Emit(ch chan int, n int) {
	ch <- n
	ch <- n + 1
}
