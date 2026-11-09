// Command miss reaches every function: one by a call, one as a function value, one through an
// interface.
package main

import "fmt"

type greeter interface {
	greet(name string)
}

type english struct{}

func (english) greet(name string) {
	fmt.Println("hello,", name)
}

func main() {
	var g greeter = english{}
	g.greet("world")
	run(farewell, "world")
}

func run(f func(string), name string) {
	f(name)
}

func farewell(name string) {
	fmt.Println("goodbye,", name)
}
