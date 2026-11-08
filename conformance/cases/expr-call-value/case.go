package expr_call_value

// Rule holds a check as a function value.
type Rule struct {
	Check func(int) bool
}

func positive(n int) bool {
	return n > 0
}

// Allowed calls the function value a rule holds in a field on n.
func Allowed(n int) bool {
	r := Rule{Check: positive}
	return r.Check(n)
}
