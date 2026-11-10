// Package mapsnap changes a map's values while ranging over it.
package mapsnap

// Sum adds the values a range over a two-entry map reads, after the first iteration sets both
// entries to 10: whichever key comes first, Go reads 1 then 10.
func Sum() int {
	m := map[string]int{}
	m["a"] = 1
	m["b"] = 1
	s := 0
	for _, v := range m {
		s += v
		m["a"] = 10
		m["b"] = 10
	}
	return s
}
