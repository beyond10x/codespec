// Command b runs only libs/onlyb; libs/shared is linked in for its init.
package main

import _ "example.com/probe/libs/shared"
import "example.com/probe/libs/onlyb"

func main() { onlyb.Run() }
