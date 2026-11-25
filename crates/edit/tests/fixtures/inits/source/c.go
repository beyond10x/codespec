package probe

func three() int { return 3 }

func init() { Registry["c"] = three() }
