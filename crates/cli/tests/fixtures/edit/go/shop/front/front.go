// Package front shows the stock.
package front

import "example.com/shop/stock"

// Count gives the number of items to show.
func Count() int {
	return stock.Load() * 2
}

// Heavy gives the weight of the shelves.
func Heavy() int {
	return stock.Weight(stock.Load())
}

// Refresh counts again in the background.
func Refresh() {
	go stock.Total()
}
