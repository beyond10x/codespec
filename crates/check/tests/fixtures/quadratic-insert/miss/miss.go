// Package miss parses parameters with an Add that appends without a scan, and scans only once
// per parse.
package miss

import "strings"

// KV is one key and value.
type KV struct {
	K string
	V string
}

// Params is a parameter list.
type Params []KV

func (p Params) index(k string) int {
	for i, kv := range p {
		if kv.K == k {
			return i
		}
	}
	return -1
}

// Add appends k and v.
func (p *Params) Add(k string, v string) {
	*p = append(*p, KV{K: k, V: v})
}

// Parse reads "k=v;k=v" and reports whether it holds a key "q".
func Parse(s string) (Params, bool) {
	var p Params
	for _, part := range strings.Split(s, ";") {
		k, v, _ := strings.Cut(part, "=")
		p.Add(k, v)
	}
	return p, p.index("q") >= 0
}
