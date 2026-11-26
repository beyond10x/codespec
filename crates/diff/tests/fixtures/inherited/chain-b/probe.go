package probe

import "example.org/lib"

func Lib() int { return lib.Do() }

func A() int { return B() }
func B() int { return Lib() + 1 }
