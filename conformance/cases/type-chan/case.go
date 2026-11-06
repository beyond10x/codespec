package type_chan

// Pipe sends n values on a channel.
func Pipe(ch chan string, n int) {
	for i := 0; i < n; i++ {
		ch <- "v"
	}
}
