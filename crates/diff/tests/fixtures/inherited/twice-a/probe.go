package probe

func Lib() int { return 1 }

func A() int { return Lib() + Lib() }
func B() int { return Lib() + 1 }
func C() int { return Lib() + 2 }
