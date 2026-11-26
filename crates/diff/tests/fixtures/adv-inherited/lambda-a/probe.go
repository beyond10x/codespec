package main

func Lib() int { return 1 }

func A() int { return func() int { return Lib() }() }
func B() int { return Lib() + 1 }

func main() { A(); B() }
