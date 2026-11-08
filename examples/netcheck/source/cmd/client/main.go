// Command client probes the server and prints how many addresses are up.
package main

import (
	"fmt"
	"strconv"

	"example.com/netcheck"
)

func main() {
	up := netcheck.CheckAll(netcheck.TCP(), "127.0.0.1:7070")
	fmt.Println("up: " + strconv.Itoa(up))
}
