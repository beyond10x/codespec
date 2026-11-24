// Package api serves orders.
package api

import "example.com/orders"

// Lines counts the lines twice.
func Lines() int {
	n := orders.Load()
	return n * 2
}
