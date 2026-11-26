package main

func A() int { return func() int { return 1 }() }

func W(f func() int) int { return f() }

func main() { A(); W(func() int { return 0 }) }
