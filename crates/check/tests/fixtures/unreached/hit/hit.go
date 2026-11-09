// Command hit has a function nothing calls.
package main

import "fmt"

func main() {
	greet("world")
}

func greet(name string) {
	fmt.Println("hello,", name)
}

func farewell(name string) {
	fmt.Println("goodbye,", name)
}
