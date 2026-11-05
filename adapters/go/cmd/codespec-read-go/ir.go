package main

// The frontend's working model: what front.go builds and encode.go turns into cir/1. It is
// internal to this adapter; cir/1 itself is defined by the Rust crate b10x-codespec-model.

type Module struct {
	Name     string      `json:"name"`
	Doc      string      `json:"doc,omitempty"`
	Types    []TypeDecl  `json:"types"`
	Consts   []ConstDecl `json:"consts,omitempty"`
	Errors   []ErrorVar  `json:"errors,omitempty"`
	Funcs    []Func      `json:"funcs"`
	Layout   []string    `json:"layout"` // source declaration order, for emitters that keep it
	Refusals []Refusal   `json:"refusals,omitempty"`
}

// Type is a type expression.
//
//	int(bits) bool string unit error any
//	named(name)            a declared type
//	ptr(elem)              a reference to elem
//	slice(elem) map(key, elem) chan(elem, dir) tuple(elems)
type Type struct {
	K     string `json:"k"`
	Name  string `json:"name,omitempty"`
	Bits  int    `json:"bits,omitempty"`
	Dir   string `json:"dir,omitempty"` // chan: "send" | "recv" | "both"
	Key   *Type  `json:"key,omitempty"`
	Elem  *Type  `json:"elem,omitempty"`
	Elems []Type `json:"elems,omitempty"`
}

// TypeDecl declares a named type.
//
//	struct   Fields
//	newtype  Under: a basic type given a name
//	enum     Under + Variants: a newtype whose values are exactly the declared constants
type TypeDecl struct {
	Name     string    `json:"name"`
	K        string    `json:"k"`
	Exported bool      `json:"exported"`
	Under    *Type     `json:"under,omitempty"`
	Fields   []Field   `json:"fields,omitempty"`
	Variants []Variant `json:"variants,omitempty"`
	Doc      string    `json:"doc,omitempty"`
	Pos      string    `json:"pos"`
}

type Field struct {
	Name string `json:"name"`
	T    Type   `json:"t"`
}

type Variant struct {
	Name  string `json:"name"`
	Value Expr   `json:"value"`
}

type ConstDecl struct {
	Name  string `json:"name"`
	T     Type   `json:"t"`
	Value Expr   `json:"value"`
	Pos   string `json:"pos"`
}

// ErrorVar is a sentinel error value: a package-level error built from a fixed message.
type ErrorVar struct {
	Name    string `json:"name"`
	Message string `json:"message"`
	Pos     string `json:"pos"`
}

type Param struct {
	Name string `json:"name"`
	T    Type   `json:"t"`
}

// Func is a function or, with Recv, a method. Fallible: the last result is an error and every
// return either has a nil error (Ok) or zero values beside a non-nil error (Err).
type Func struct {
	Name     string  `json:"name"`
	Exported bool    `json:"exported"`
	Recv     *Param  `json:"recv,omitempty"`
	Params   []Param `json:"params"`
	Results  []Type  `json:"results"`
	Body     []Stmt  `json:"body"`
	Pos      string  `json:"pos"`
	Doc      string  `json:"doc,omitempty"`
}

// Stmt is a statement.
//
//	let          Names := Rhs                     (Names[i] "_" is a discard)
//	var          Names[0] T                       zero value
//	assign       Lhs = Rhs, Op "" or "+=" etc.
//	incr / decr  X++ / X--
//	if           Cond Then Else
//	for_range    for Key, Val := range Over { Body }
//	return       Results
//	expr         X
//	send         Chan <- X
//	lookup_else  Names[0] := Map[Key], else Then     (pass "lookup-guard")
//	foreign      Lang, Src: source the frontend kept verbatim, with Reason
type Stmt struct {
	K       string   `json:"k"`
	Names   []string `json:"names,omitempty"`
	T       *Type    `json:"t,omitempty"`
	Op      string   `json:"op,omitempty"`
	Lhs     []Expr   `json:"lhs,omitempty"`
	Rhs     []Expr   `json:"rhs,omitempty"`
	X       *Expr    `json:"x,omitempty"`
	Chan    *Expr    `json:"chan,omitempty"`
	Cond    *Expr    `json:"cond,omitempty"`
	Then    []Stmt   `json:"then,omitempty"`
	Else    []Stmt   `json:"else,omitempty"`
	Key     string   `json:"key,omitempty"`
	Val     string   `json:"val,omitempty"`
	Over    *Expr    `json:"over,omitempty"`
	Map     *Expr    `json:"map,omitempty"`
	Index   *Expr    `json:"index,omitempty"`
	Body    []Stmt   `json:"body,omitempty"`
	Results []Expr   `json:"results,omitempty"`
	Lang    string   `json:"lang,omitempty"`
	Src     string   `json:"src,omitempty"`
	Reason  string   `json:"reason,omitempty"`
	Pos     string   `json:"pos,omitempty"`
}

// Expr is an expression. T is its type where the frontend knows it.
//
//	lit      Value (Go literal spelling), T
//	nil      T
//	var      Name: local, parameter or receiver
//	const    Name: a declared constant or enum variant
//	errvar   Name: a sentinel error
//	field    X.Name
//	index    X[Y]
//	call     Name(Args): a module function
//	method   X.Name(Args)
//	binop    X Op Y
//	unop     Op X     ("!", "-", "*" deref, "&" addr)
//	struct   Name{Fields}  (Args[i] value of FieldNames[i])
//	conv     Name(X): conversion to a named type
//	zero     T
//	make     T: an empty map or slice
//	intrinsic Name(Args): sprintf, errorf, println, append, len
//	to_error X: a value of an error struct used as error
type Expr struct {
	K          string   `json:"k"`
	T          *Type    `json:"t,omitempty"`
	Name       string   `json:"name,omitempty"`
	Value      string   `json:"value,omitempty"`
	Op         string   `json:"op,omitempty"`
	X          *Expr    `json:"x,omitempty"`
	Y          *Expr    `json:"y,omitempty"`
	Args       []Expr   `json:"args,omitempty"`
	FieldNames []string `json:"field_names,omitempty"`
}

type Refusal struct {
	Pos    string `json:"pos"`
	What   string `json:"what"`
	Reason string `json:"reason"`
}

func T(k string) Type { return Type{K: k} }

func has(xs []string, x string) bool {
	for _, y := range xs {
		if y == x {
			return true
		}
	}
	return false
}
