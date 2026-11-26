package main

func Lib() int { return 1 }

func A() int {
	f := Lib
	return f()
}

func main() { A() }
