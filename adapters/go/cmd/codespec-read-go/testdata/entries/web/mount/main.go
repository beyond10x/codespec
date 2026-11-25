package main

import "net/http"

type appHandler func(w http.ResponseWriter, r *http.Request) error

func (fn appHandler) ServeHTTP(w http.ResponseWriter, r *http.Request) {}

func view(w http.ResponseWriter, r *http.Request) error { return nil }
func plain(w http.ResponseWriter, r *http.Request)      {}

func main() {
	api := http.NewServeMux()
	http.Handle("/view", appHandler(view))
	h := appHandler(view)
	http.Handle("/held", h)
	http.Handle("/plain", http.HandlerFunc(plain))
	http.Handle("/api/", api)
	http.Handle("/sub/", http.NewServeMux())
	http.ListenAndServe(":8080", nil)
}
