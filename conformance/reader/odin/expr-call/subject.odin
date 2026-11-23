package subject

@(private)
twice :: proc(n: int) -> int {
	return n + n
}

Quad :: proc(n: int) -> int {
	return twice(twice(n))
}
