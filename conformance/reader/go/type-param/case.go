package type_param

func second[T any](x T, y T) T {
	return y
}

// Second gives the second of two numbers through a function over any type.
func Second(a int, b int) int {
	return second[int](a, b)
}
