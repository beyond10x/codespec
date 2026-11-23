package subject

Sign :: proc(n: int) -> int {
	if n > 0 {
		return 1
	} else if n < 0 {
		return 2
	}
	return 0
}
