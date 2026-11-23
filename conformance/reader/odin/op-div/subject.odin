package subject

Div :: proc(a: int, b: int) -> int {
	if b == -1 {
		return -a
	}
	return a / b
}
