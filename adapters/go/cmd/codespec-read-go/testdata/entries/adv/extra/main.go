// Package main registers routes with echo v4's other registrations: Add, Match, CONNECT and
// TRACE.
package main

import "github.com/labstack/echo/v4"

func one(c echo.Context) error   { return nil }
func two(c echo.Context) error   { return nil }
func three(c echo.Context) error { return nil }
func four(c echo.Context) error  { return nil }

func main() {
	e := echo.New()
	e.Add("GET", "/added", one)
	e.Match([]string{"GET", "POST"}, "/matched", two)
	e.CONNECT("/tunnel", three)
	e.TRACE("/trace", four)
	e.Start(":8080")
}
