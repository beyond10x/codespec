package expr_extern

import (
	"strconv"
	"strings"
)

// Banner repeats s n times and appends n: two calls of standard library functions.
func Banner(s string, n int) string {
	return strings.Repeat(s, n) + strconv.Itoa(n)
}
