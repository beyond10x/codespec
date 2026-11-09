// Package hit parses a port into 16 bits without checking its range.
package hit

import "strconv"

// Port is a TCP or UDP port.
type Port uint16

// ParsePort reads a decimal port.
func ParsePort(s string) (Port, error) {
	v, err := strconv.Atoi(s)
	if err != nil {
		return 0, err
	}
	return Port(v), nil
}
