package probe

// Registry holds the values the init functions register.
var Registry = map[string]int{}

// T is closed once.
type T struct{ n int }

// Close closes t.
func (t *T) Close() int { return t.n }

func one() int { return 1 }

func init() { Registry["a"] = one() }

func _() { one() }
