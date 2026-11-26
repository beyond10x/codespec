package main

import "example.org/lib"

func Lib() int { return lib.Do() }

func A() int { return func() int { return Lib() }() }
func B() int { return Lib() + 1 }

func main() { A(); B() }
