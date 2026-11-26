package probe

import "example.org/lib"

func Lib() int { return lib.Do() }

func A() int { return Lib() + lib.Other() }
func B() int { return Lib() + 1 }
func C() int { return Lib() + 2 }
