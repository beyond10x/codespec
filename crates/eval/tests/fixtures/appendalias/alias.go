// Package alias holds two slices over one array: Set appends to the shorter one, which writes
// the longer one's last element in place, then sends.
package alias

// S keeps base, five ints whose capacity Go rounds up to six (the 48-byte size class), and
// sixth, base with one more element in the same array.
type S struct {
	base  []int
	sixth []int
	out   chan int
}

// New gives an S whose sixth shares the backing array of its base.
func New(out chan int) *S {
	var base []int
	base = append(base, 1, 2, 3, 4, 5)
	sixth := append(base, 0)
	return &S{base: base, sixth: sixth, out: out}
}

// Sixth reports the sixth element.
func (s *S) Sixth() int {
	return s.sixth[5]
}

// Set appends 7 to base, which overwrites sixth[5] in place, and sends the new length.
func (s *S) Set() {
	b := append(s.base, 7)
	s.out <- len(b)
}
