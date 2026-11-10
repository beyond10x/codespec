// Package spin loops forever with an empty body, which only the step budget can end.
package spin

// Spin never returns.
func Spin() {
	for {
	}
}
