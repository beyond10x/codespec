package sem_switch_break

// Until counts up to n but stops at stop; the break leaves the loop, not only the switch.
func Until(n int, stop int) int {
	i := 0
loop:
	for i < n {
		switch i {
		case stop:
			break loop
		}
		i++
	}
	return i
}
