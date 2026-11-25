package probe

// U is closed once.
type U struct{ m int }

// Close closes u.
func (u *U) Close() int { return u.m }

func two() int { return 2 }

func init() { Registry["b"] = two() }

func _() { two() }
