package probe

func Lib() int { return 1 }

func A() int { return B() }
func B() int { return Lib() + 1 }
