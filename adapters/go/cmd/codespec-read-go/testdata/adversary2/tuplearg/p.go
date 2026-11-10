package tuplearg

func two() (int, int) { return 1, 2 }

func add(a int, b int) int { return a*10 + b }

func sum(xs ...int) int {
	s := 0
	for _, x := range xs {
		s += x
	}
	return s
}

// Spread passes both results of two to a function of two parameters: Go gives 12.
func Spread() int { return add(two()) }

// Variadic passes both results of two to a variadic function: Go gives 3.
func Variadic() int { return sum(two()) }
