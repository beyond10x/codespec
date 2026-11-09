// Package ranged parses ports and status codes into int with no range check in three
// functions, and with one in a fourth; a count has no range and is not checked.
package ranged

import (
	"errors"
	"strconv"
)

// URI is a host and a port.
type URI struct {
	Host string
	Port int
}

// parsePort sets u.Port from s.
func parsePort(u *URI, s string) error {
	var err error
	u.Port, err = strconv.Atoi(s)
	return err
}

// parseStatus reads a three-digit status code.
func parseStatus(s string) (int, error) {
	statusCode, err := strconv.ParseUint(s, 10, 16)
	return int(statusCode), err
}

// viaPort gives the port rport names, or def.
func viaPort(rport string, def int) int {
	port := def
	p, err := strconv.Atoi(rport)
	if err == nil {
		port = p
	}
	return port
}

// checkedPort refuses a port outside 1..65535.
func checkedPort(s string) (int, error) {
	port, err := strconv.Atoi(s)
	if err != nil {
		return 0, err
	}
	if port < 1 || port > 65535 {
		return 0, errors.New("port out of range")
	}
	return port, nil
}

// count reads a count.
func count(s string) (int, error) {
	n, err := strconv.Atoi(s)
	return n, err
}
