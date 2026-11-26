package probe

import "example.org/lib"

func Lib() int { return lib.Do() }

func A(n int) int {
	if n == 0 {
		return 0
	}
	return B(n - 1)
}

func B(n int) int {
	if n == 0 {
		return Lib()
	}
	return A(n - 1)
}

func C() int { return A(3) }
