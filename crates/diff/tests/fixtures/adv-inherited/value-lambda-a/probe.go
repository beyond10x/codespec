package main

func Lib() int { return 1 }

func A() int { return func() int { return Lib() }() }

func W(f func() int) int { return f() }

func main() { A(); W(func() int { return 0 }) }
