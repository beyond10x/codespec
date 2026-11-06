package expr_to_error

// NotFound is an error naming a key.
type NotFound struct {
	Key string
}

func (e NotFound) Error() string {
	return "not found: " + e.Key
}

// Get never finds a key.
func Get(k string) error {
	return NotFound{Key: k}
}
