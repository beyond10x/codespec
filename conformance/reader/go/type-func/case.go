package type_func

func twice(n int) int {
	return n * 2
}

func run(f func(int) int, n int) int {
	return f(n)
}

// Apply passes twice to run, whose parameter has a function type.
func Apply(n int) int {
	return run(twice, n)
}
