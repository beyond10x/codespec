package unop_not

// Neither is true when both are false.
func Neither(a bool, b bool) bool {
	return !a && !b
}
