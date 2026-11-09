// Package miss copies the body of both messages when it clones them.
package miss

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
	return Response{Body: slices.Clone(r.Body)}
}
