package main

import "example.org/lib"

func A() int { return func() int { return lib.Do() }() }

func W(f func() int) int { return f() }

func main() { A(); W(func() int { return 0 }) }
