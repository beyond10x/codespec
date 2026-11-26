package probe

func Lib() int { return 1 }

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
