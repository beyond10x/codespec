// Command server answers each line a client sends with netcheck.Answer.
package main

import (
	"bufio"
	"fmt"
	"io"
	"net"

	"example.com/netcheck"
)

func main() {
	ln, err := net.Listen("tcp", "127.0.0.1:7070")
	if err != nil {
		fmt.Println("listen: " + err.Error())
		return
	}
	for {
		c, err := ln.Accept()
		if err != nil {
			return
		}
		serve(c)
	}
}

// serve reads one line from c, writes the answer and closes c.
func serve(c net.Conn) {
	line, err := bufio.NewReader(c).ReadString('\n')
	if err == nil {
		io.WriteString(c, netcheck.Answer(line)+"\n")
	}
	c.Close()
}
