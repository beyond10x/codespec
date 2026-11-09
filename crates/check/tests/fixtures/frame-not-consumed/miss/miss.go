// Package miss discards control frames' payload before it reads the next header.
package miss

import "io"

// Header is a frame header.
type Header struct {
	Control bool
	Length  int64
}

// Reader reads frames.
type Reader struct {
	src io.Reader
}

// NextFrame reads the next frame header.
func (r *Reader) NextFrame() (Header, error) {
	return Header{}, nil
}

// Discard skips the current frame's payload.
func (r *Reader) Discard() error {
	return nil
}

// ReadData returns the payload of the next data frame.
func ReadData(r *Reader, src io.Reader) ([]byte, error) {
	for {
		h, err := r.NextFrame()
		if err != nil {
			return nil, err
		}
		if h.Control {
			err = r.Discard()
			if err != nil {
				return nil, err
			}
			continue
		}
		data := make([]byte, h.Length)
		_, err = io.ReadFull(src, data)
		return data, err
	}
}
