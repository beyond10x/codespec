package fact_shared

// Node is stored by reference in a map.
type Node struct {
	V int
}

// Stored keeps a node in a map and reads it back.
func Stored(v int) int {
	m := map[int]*Node{}
	m[1] = &Node{V: v}
	return m[1].V
}
