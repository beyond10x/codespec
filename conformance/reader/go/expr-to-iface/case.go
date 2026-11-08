package expr_to_iface

type Labeler interface {
	Label() string
}

type Tag struct {
	text string
}

func (t Tag) Label() string {
	return "<" + t.text + ">"
}

// Label converts a Tag holding s to a Labeler by assignment and calls through it.
func Label(s string) string {
	var l Labeler
	l = Tag{text: s}
	return l.Label()
}
