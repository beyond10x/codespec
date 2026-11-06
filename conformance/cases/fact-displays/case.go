package fact_displays

// Tag is a text label.
type Tag string

// Label converts to a Tag.
func Label(s string) Tag {
	return Tag(s)
}
