package fact_messages

// Tick is a message.
type Tick int

// Ticks sends n ticks on a channel of any.
func Ticks(ch chan any, n int) {
	for i := 0; i < n; i++ {
		ch <- Tick(i)
	}
}
