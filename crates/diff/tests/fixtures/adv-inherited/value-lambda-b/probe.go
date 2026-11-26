package main

import "example.org/lib"

func Lib() int { return lib.Do() }

func A() int { return func() int { return Lib() }() }

func W(f func() int) int { return f() }

func main() { A(); W(func() int { return 0 }) }
