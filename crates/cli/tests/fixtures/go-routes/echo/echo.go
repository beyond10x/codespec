// Package echo is a stub of github.com/labstack/echo/v4 with the same API shape for routes, so
// the reader's tests resolve it offline.
package echo

type Context interface {
	String(code int, s string) error
}

type HandlerFunc func(c Context) error

type MiddlewareFunc func(next HandlerFunc) HandlerFunc

type Route struct {
	Method string
	Path   string
}

type Echo struct{}

type Group struct {
	prefix string
}

func New() *Echo { return &Echo{} }

func (e *Echo) Start(addr string) error { return nil }

func (e *Echo) Use(m ...MiddlewareFunc) {}

func (e *Echo) Group(prefix string, m ...MiddlewareFunc) *Group { return &Group{prefix: prefix} }

func (e *Echo) GET(path string, h HandlerFunc, m ...MiddlewareFunc) *Route     { return nil }
func (e *Echo) POST(path string, h HandlerFunc, m ...MiddlewareFunc) *Route    { return nil }
func (e *Echo) PUT(path string, h HandlerFunc, m ...MiddlewareFunc) *Route     { return nil }
func (e *Echo) DELETE(path string, h HandlerFunc, m ...MiddlewareFunc) *Route  { return nil }
func (e *Echo) PATCH(path string, h HandlerFunc, m ...MiddlewareFunc) *Route   { return nil }
func (e *Echo) HEAD(path string, h HandlerFunc, m ...MiddlewareFunc) *Route    { return nil }
func (e *Echo) OPTIONS(path string, h HandlerFunc, m ...MiddlewareFunc) *Route { return nil }
func (e *Echo) Any(path string, h HandlerFunc, m ...MiddlewareFunc) []*Route   { return nil }

func (g *Group) Group(prefix string, m ...MiddlewareFunc) *Group {
	return &Group{prefix: g.prefix + prefix}
}

func (g *Group) GET(path string, h HandlerFunc, m ...MiddlewareFunc) *Route     { return nil }
func (g *Group) POST(path string, h HandlerFunc, m ...MiddlewareFunc) *Route    { return nil }
func (g *Group) PUT(path string, h HandlerFunc, m ...MiddlewareFunc) *Route     { return nil }
func (g *Group) DELETE(path string, h HandlerFunc, m ...MiddlewareFunc) *Route  { return nil }
func (g *Group) PATCH(path string, h HandlerFunc, m ...MiddlewareFunc) *Route   { return nil }
func (g *Group) HEAD(path string, h HandlerFunc, m ...MiddlewareFunc) *Route    { return nil }
func (g *Group) OPTIONS(path string, h HandlerFunc, m ...MiddlewareFunc) *Route { return nil }
func (g *Group) Any(path string, h HandlerFunc, m ...MiddlewareFunc) []*Route   { return nil }
