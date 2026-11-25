// Package echo is a stub of github.com/labstack/echo/v4 with the API shape of every route
// registration echo v4 has: the method shortcuts, Any, Add, Match, and groups.
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

func (e *Echo) Group(prefix string, m ...MiddlewareFunc) *Group { return &Group{prefix: prefix} }

func (e *Echo) GET(path string, h HandlerFunc, m ...MiddlewareFunc) *Route     { return nil }
func (e *Echo) POST(path string, h HandlerFunc, m ...MiddlewareFunc) *Route    { return nil }
func (e *Echo) CONNECT(path string, h HandlerFunc, m ...MiddlewareFunc) *Route { return nil }
func (e *Echo) TRACE(path string, h HandlerFunc, m ...MiddlewareFunc) *Route   { return nil }
func (e *Echo) Any(path string, h HandlerFunc, m ...MiddlewareFunc) []*Route   { return nil }

func (e *Echo) Add(method, path string, h HandlerFunc, m ...MiddlewareFunc) *Route { return nil }

func (e *Echo) Match(methods []string, path string, h HandlerFunc, m ...MiddlewareFunc) []*Route {
	return nil
}

func (g *Group) Group(prefix string, m ...MiddlewareFunc) *Group {
	return &Group{prefix: g.prefix + prefix}
}

func (g *Group) GET(path string, h HandlerFunc, m ...MiddlewareFunc) *Route { return nil }
