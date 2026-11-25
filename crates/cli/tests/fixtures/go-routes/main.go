// Package main registers routes the Go reader cannot turn into entries, each beside one it can.
package main

import (
	"net/http"

	"github.com/labstack/echo/v4"
)

const base = "/v1"

type config struct {
	Prefix string
}

type route struct {
	path string
	h    echo.HandlerFunc
}

func ping(c echo.Context) error { return nil }

// mount registers on a group passed in: its prefix is not known here.
func mount(g *echo.Group) {
	g.GET("/mounted", ping)
}

func main() {
	e := echo.New()
	cfg := config{Prefix: "/cfg"}
	routes := []route{{path: "/a", h: ping}, {path: "/b", h: ping}}
	for _, r := range routes {
		e.GET(r.path, r.h)
	}
	e.GET(cfg.Prefix+"/x", ping)
	e.GET("/lit", func(c echo.Context) error { return nil })
	h := ping
	e.GET("/var", h)
	var nf http.Handler = http.NotFoundHandler()
	http.Handle("/iface", nf)
	twice := e.Group("/one")
	twice = e.Group("/two")
	twice.GET("/twice", ping)
	mount(e.Group("/m"))
	e.GET(base+"/named", ping)
	e.Start(":8080")
}
