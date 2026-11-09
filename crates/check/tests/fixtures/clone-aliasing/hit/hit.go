// Package hit copies a request's body when it clones one, but shares a response's.
package hit

import "slices"

// Request is a message with a body.
type Request struct {
	Body []byte
}

// Response is a message with a body.
type Response struct {
	Body []byte
}

func cloneRequest(r Request) Request {
	return Request{Body: slices.Clone(r.Body)}
}

func cloneResponse(r Response) Response {
	return Response{Body: r.Body}
}
