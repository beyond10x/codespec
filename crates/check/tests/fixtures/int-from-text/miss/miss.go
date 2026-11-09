// Package miss parses a port into 16 bits after checking its range, or with a 16-bit parse.
package miss

import (
	"errors"
	"strconv"
)

// Port is a TCP or UDP port.
type Port uint16

// ParsePort reads a decimal port and refuses one outside 0..65535.
func ParsePort(s string) (Port, error) {
	v, err := strconv.Atoi(s)
	if err != nil {
		return 0, err
	}
	if v < 0 || v > 65535 {
		return 0, errors.New("port out of range")
	}
	return Port(v), nil
}

// ParsePort16 reads a decimal port with a parse that refuses more than 16 bits.
func ParsePort16(s string) (Port, error) {
	v, err := strconv.ParseUint(s, 10, 16)
	if err != nil {
		return 0, err
	}
	return Port(v), nil
}
