package sem_continue_post

// SkipOne sums 0 up to n-1 except skip; the continue still runs i++.
func SkipOne(n int, skip int) int {
	s := 0
	for i := 0; i < n; i++ {
		if i == skip {
			continue
		}
		s += i
	}
	return s
}
