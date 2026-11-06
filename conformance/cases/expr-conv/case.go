package expr_conv

// ID is a key.
type ID int

// Wrap converts to ID.
func Wrap(n int) ID {
	return ID(n)
}
