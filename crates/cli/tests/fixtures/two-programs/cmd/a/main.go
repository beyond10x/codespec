// Command a runs only libs/onlya; libs/shared is linked in for its init.
package main

import _ "example.com/probe/libs/shared"
import "example.com/probe/libs/onlya"

func main() { onlya.Run() }
