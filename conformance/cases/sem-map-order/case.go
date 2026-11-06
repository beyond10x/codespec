package sem_map_order

// Squares sums the squares of 0 up to n-1 through a map; the sum is the same in any order.
func Squares(n int) int {
	m := map[int]int{}
	for i := 0; i < n; i++ {
		m[i] = i * i
	}
	s := 0
	for _, v := range m {
		s += v
	}
	return s
}
