package type_slice

// Pair is a slice of two strings.
func Pair(a string, b string) []string {
	xs := []string{}
	xs = append(xs, a)
	xs = append(xs, b)
	return xs
}
