// Package main registers a function converted to a named function type with a ServeHTTP of its
// own: net/http runs that ServeHTTP, which calls the function and handles its error.
package main

import "net/http"

type appHandler func(w http.ResponseWriter, r *http.Request) error

func (fn appHandler) ServeHTTP(w http.ResponseWriter, r *http.Request) {
	if err := fn(w, r); err != nil {
		http.Error(w, err.Error(), 500)
	}
}

func view(w http.ResponseWriter, r *http.Request) error { return nil }

func main() {
	http.Handle("/view", appHandler(view))
	http.ListenAndServe(":8080", nil)
}
