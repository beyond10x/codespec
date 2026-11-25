package main

import (
	"example.com/shop/handlers"
	"github.com/labstack/echo/v4"
)

func main() {
	e := echo.New()
	e.GET("/health", handlers.Health)
	api := e.Group("/api")
	h := handlers.Items{}
	api.GET("/items", h.List)
	api.GET("/items/:id", h.Show)
	e.Start(":8080")
}
