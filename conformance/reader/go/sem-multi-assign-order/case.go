package sem_multi_assign_order

// Swap exchanges two values.
func Swap(a int, b int) (int, int) {
	a, b = b, a
	return a, b
}

// Fib is the n-th Fibonacci number.
func Fib(n int) int {
	a := 0
	b := 1
	for i := 0; i < n; i++ {
		a, b = b, a+b
	}
	return a
}
