package type_map

// Stored puts the length of a string under it and reads it back.
func Stored(s string) int {
	m := map[string]int{}
	m[s] = len(s)
	return m[s]
}
