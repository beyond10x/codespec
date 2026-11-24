// Package front shows the stock of the depot.
package front

import "example.com/depot/stock"

// Count gives the number of goods to show.
func Count() int {
	return stock.Load() * 2
}

// Refresh counts again in the background.
func Refresh() {
	go stock.Total()
}
