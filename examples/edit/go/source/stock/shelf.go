package stock

// Sized is anything that has a size.
type Sized interface {
	Size() int
}

// Shelf holds goods in slots.
type Shelf struct {
	Slots int
}

// Size gives the number of slots.
func (s Shelf) Size() int {
	return s.Slots
}
