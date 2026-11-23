package badimport

import "example.com/nowhere"

// F calls a package no module provides.
func F() int { return nowhere.F() }
