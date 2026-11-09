// Package hit parses parameters with an Add that scans every parameter it holds: parsing n
// parameters costs n².
package hit

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

// Add sets k to v, replacing an existing k.
func (p *Params) Add(k string, v string) {
	i := p.index(k)
	if i >= 0 {
		(*p)[i].V = v
		return
	}
	*p = append(*p, KV{K: k, V: v})
}

// Parse reads "k=v;k=v".
func Parse(s string) Params {
	var p Params
	for _, part := range strings.Split(s, ";") {
		k, v, _ := strings.Cut(part, "=")
		p.Add(k, v)
	}
	return p
}
