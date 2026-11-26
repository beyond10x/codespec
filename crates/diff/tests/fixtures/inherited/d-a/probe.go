package probe

func Lib() int { return 1 }

func A() int { return Lib() }
func B() int { return Lib() + 1 }
func C() int { return Lib() + 2 }
func D() int { return 3 }
