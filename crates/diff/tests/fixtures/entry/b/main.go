package main

import "github.com/labstack/echo/v4"

func list(c echo.Context) error { return nil }

func create(c echo.Context) error { return nil }

func main() {
	e := echo.New()
	e.GET("/items", list)
	e.POST("/items", create)
	e.Start(":8080")
}
