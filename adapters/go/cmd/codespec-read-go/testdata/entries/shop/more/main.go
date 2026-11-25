package main

import (
	"os"

	"github.com/labstack/echo/v4"
)

type api struct {
	*echo.Group
}

type server struct {
	*echo.Echo
}

func one(c echo.Context) error   { return nil }
func two(c echo.Context) error   { return nil }
func three(c echo.Context) error { return nil }
func four(c echo.Context) error  { return nil }

func main() {
	e := echo.New()
	e.Add("PUT", "/added", one)
	e.Match([]string{"GET", "HEAD"}, "/matched", two)
	e.CONNECT("/tunnel", three)
	g := e.Group("/g")
	g.TRACE("/trace", four)
	method := os.Getenv("METHOD")
	e.Add(method, "/dynamic", one)
	methods := []string{"GET"}
	e.Match(methods, "/held", two)
	e.Match([]string{"GET", method}, "/mixed", two)
	a := api{e.Group("/api")}
	a.GET("/items", one)
	s := server{e}
	s.GET("/served", one)
	e.Start(":8080")
}
