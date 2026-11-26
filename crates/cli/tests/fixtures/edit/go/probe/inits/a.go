package inits

// Registry holds what the init functions register.
var Registry = map[string]int{}

func one() int { return 1 }

func two() int { return 2 }

func init() { Registry["a"] = one() }
