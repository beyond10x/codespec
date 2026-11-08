package decl_generic

// Sizer has a size.
type Sizer interface {
	Size() int
}

type Item struct {
	n int
}

func (i Item) Size() int {
	return i.n
}

// Pair holds two values of one type.
type Pair[T any] struct {
	a T
	b T
}

func (p Pair[T]) First() T {
	return p.a
}

func sum[T Sizer](x T, y T) int {
	return x.Size() + y.Size()
}

// Sizes makes a pair of items of size n and n+1 and sums their sizes through Sizer.
func Sizes(n int) int {
	p := Pair[Item]{a: Item{n: n}, b: Item{n: n + 1}}
	return sum[Item](p.First(), p.b)
}
