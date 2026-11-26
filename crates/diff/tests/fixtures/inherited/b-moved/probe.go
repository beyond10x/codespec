package probe

import "example.org/lib"

// Lib reads the library.
// It is incomplete in both revisions.
func Lib() int { return lib.Do() }

func A() int { return Lib() }
func B() int { return Lib() + 1 }
func C() int { return Lib() + 2 }
