package main

import "example.org/lib"

func Lib() int { return lib.Do() }

func A() int {
	f := Lib
	return f()
}

func main() { A() }
