package main

import "github.com/labstack/echo/v4"

func all(c echo.Context) error    { return nil }
func create(c echo.Context) error { return nil }
func drop(c echo.Context) error   { return nil }
func stats(c echo.Context) error  { return nil }

func auth(next echo.HandlerFunc) echo.HandlerFunc { return next }

const v2 = "/v2"

func main() {
	e := echo.New()
	e.Any("/all", all)
	api := e.Group("/api", auth)
	items := api.Group(v2 + "/items")
	items.POST("", create, auth)
	items.DELETE("/:id", drop)
	e.Group("/admin").GET("/stats", stats)
	e.Start(":8080")
}
