// Package main registers a route on a struct that embeds an *echo.Group: the route's path is the
// group's prefix joined with the route's.
package main

import "github.com/labstack/echo/v4"

type api struct {
	*echo.Group
}

func list(c echo.Context) error { return nil }

func main() {
	e := echo.New()
	a := api{e.Group("/api")}
	a.GET("/items", list)
	e.Start(":8080")
}
