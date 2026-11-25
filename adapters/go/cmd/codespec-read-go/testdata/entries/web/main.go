package main

import "net/http"

type static struct{}

func (static) ServeHTTP(w http.ResponseWriter, r *http.Request) {}

func show(w http.ResponseWriter, r *http.Request)   {}
func health(w http.ResponseWriter, r *http.Request) {}
func create(w http.ResponseWriter, r *http.Request) {}

func main() {
	mux := http.NewServeMux()
	mux.HandleFunc("GET /items/{id}", show)
	http.HandleFunc("/health", health)
	var files static
	http.Handle("/static/", files)
	mux.Handle("POST /items", http.HandlerFunc(create))
	http.ListenAndServe(":8080", mux)
}
