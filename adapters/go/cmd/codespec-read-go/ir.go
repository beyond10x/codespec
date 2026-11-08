package main

// The frontend's working model: what front.go builds and encode.go turns into cir/1. It is
// internal to this adapter; cir/1 itself is defined by the Rust crate b10x-codespec-model.

type Module struct {
	Name     string      `json:"name"`
	Doc      string      `json:"doc,omitempty"`
	Types    []TypeDecl  `json:"types"`
	Consts   []ConstDecl `json:"consts,omitempty"`
	Errors   []ErrorVar  `json:"errors,omitempty"`
	Vars     []VarDecl   `json:"vars,omitempty"`
	Funcs    []Func      `json:"funcs"`
	Externs  []Extern    `json:"externs,omitempty"` // candidates; the encoder emits those a call uses
	Layout   []string    `json:"layout"`            // source declaration order, for emitters that keep it
	Refusals []Refusal   `json:"refusals,omitempty"`
	// For a whole-program analysis: the import path, every function not in Funcs, the method set of
	// each named type, and every //go:linkname of a function (calls.go).
	Path      string      `json:"path,omitempty"`
	Opaque    []Opaque    `json:"opaque,omitempty"`
	Methods   []MethodSet `json:"methods,omitempty"`
	Linknames []Linkname  `json:"linknames,omitempty"`
	Inits     []CallRef   `json:"inits,omitempty"`
	// Interface types: the package's own in source order, then those of other packages a call
	// goes through (candidates; the encoder emits those an interface call uses).
	Interfaces []Interface `json:"interfaces,omitempty"`
}

// Interface is an interface type with its whole method set, sorted by name; with Path, one of
// another package.
type Interface struct {
	Path     string        `json:"path,omitempty"`
	Name     string        `json:"name"`
	Exported bool          `json:"exported"`
	Methods  []IfaceMethod `json:"methods"`
	Doc      string        `json:"doc,omitempty"`
	Pos      string        `json:"pos,omitempty"`
}

type IfaceMethod struct {
	Name    string `json:"name"`
	Params  []Type `json:"params"`
	Results []Type `json:"results"`
}

// Type is a type expression.
//
//	int(bits) bool string unit error any
//	named(name)            a declared type
//	extern(path, name)     an opaque type of another package; named and extern carry Targs when instantiated
//	ptr(elem)              a reference to elem
//	slice(elem) map(key, elem) chan(elem, dir) tuple(elems)
//	func(params, results)
//	param(name)            a type parameter of the enclosing generic declaration
type Type struct {
	K        string `json:"k"`
	Name     string `json:"name,omitempty"`
	Path     string `json:"path,omitempty"` // extern: the package declaring Name
	Bits     int    `json:"bits,omitempty"`
	Len      int64  `json:"len,omitempty"` // array
	Unsigned bool   `json:"unsigned,omitempty"`
	Dir      string `json:"dir,omitempty"` // chan: "send" | "recv" | "both"
	Key      *Type  `json:"key,omitempty"`
	Elem     *Type  `json:"elem,omitempty"`
	Elems    []Type `json:"elems,omitempty"`
	Params   []Type `json:"params,omitempty"`  // func
	Results  []Type `json:"results,omitempty"` // func
	Targs    []Type `json:"targs,omitempty"`   // named, extern: the type arguments of an instantiation
}

// TypeDecl declares a named type.
//
//	struct   Fields
//	newtype  Under: a basic type given a name
//	enum     Under + Variants: a newtype whose values are exactly the declared constants
type TypeDecl struct {
	Name     string      `json:"name"`
	K        string      `json:"k"`
	Exported bool        `json:"exported"`
	Tparams  []TypeParam `json:"tparams,omitempty"`
	Under    *Type       `json:"under,omitempty"`
	Fields   []Field     `json:"fields,omitempty"`
	Variants []Variant   `json:"variants,omitempty"`
	Doc      string      `json:"doc,omitempty"`
	Pos      string      `json:"pos"`
}

type Field struct {
	Name     string `json:"name"`
	T        Type   `json:"t"`
	Embedded bool   `json:"embedded,omitempty"`
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

// VarDecl is a package-level variable: the zero value of T, the value of Init, or an initializer
// kept as Foreign Go source.
type VarDecl struct {
	Name    string `json:"name"`
	T       Type   `json:"t"`
	Init    *Expr  `json:"init,omitempty"`
	Foreign *Stmt  `json:"foreign,omitempty"` // Lang, Src, Reason
	Pos     string `json:"pos"`
}

// Extern is a function of another package that the module calls; with Recv, a method of a type of
// that package.
type Extern struct {
	Path    string `json:"path"`
	Name    string `json:"name"`
	Recv    *Type  `json:"recv,omitempty"`
	Params  []Type `json:"params"`
	Results []Type `json:"results"`
	// Variadic: the last of Params is variadic, a slice inside the callee.
	Variadic bool        `json:"variadic,omitempty"`
	Tparams  []TypeParam `json:"tparams,omitempty"` // the function's own type parameters
}

type Param struct {
	Name string `json:"name"`
	T    Type   `json:"t"`
	// Variadic: the last parameter of a function, a slice inside the body (Go `...T`).
	Variadic bool `json:"variadic,omitempty"`
}

// Func is a function or, with Recv, a method. Fallible: the last result is an error and every
// return either has a nil error (Ok) or zero values beside a non-nil error (Err).
type Func struct {
	Name     string      `json:"name"`
	Exported bool        `json:"exported"`
	Tparams  []TypeParam `json:"tparams,omitempty"`
	Recv     *Param      `json:"recv,omitempty"`
	Params   []Param     `json:"params"`
	Results  []Type      `json:"results"`
	Body     []Stmt      `json:"body"`
	Pos      string      `json:"pos"`
	Doc      string      `json:"doc,omitempty"`
}

// Stmt is a statement.
//
//	let          Names := Rhs                     (Names[i] "_" is a discard)
//	var          Names[0] T                       zero value
//	assign       Lhs = Rhs, Op "" or "+=" etc.
//	incr / decr  X++ / X--
//	if           Cond Then Else
//	for_range    for Key, Val := range Over { Body }
//	for          for Init; Cond; Post { Body }       (each of Init, Cond, Post optional)
//	break, continue                                (unlabeled, of the innermost loop)
//	switch       switch Tag { Cases }             (Tag nil: switch true)
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
	Init    *Stmt    `json:"init,omitempty"`
	Post    *Stmt    `json:"post,omitempty"`
	Tag     *Expr    `json:"tag,omitempty"`
	Cases   []Case   `json:"cases,omitempty"`
	Lang    string   `json:"lang,omitempty"`
	Src     string   `json:"src,omitempty"`
	Reason  string   `json:"reason,omitempty"`
	Pos     string   `json:"pos,omitempty"`
	// foreign: every call the source makes (calls.go)
	Calls []CallRef `json:"calls,omitempty"`
}

// Expr is an expression. T is its type where the frontend knows it.
//
//	lit      Value (Go literal spelling), T
//	nil      T
//	var      Name: local, parameter or receiver
//	const    Name: a declared constant or enum variant
//	errvar   Name: a sentinel error
//	global   Name: a package-level variable, declared in Vars
//	field    X.Name
//	index    X[Y]
//	call     Name(Args): a module function
//	method   X.Name(Args)
//	extern   Path.Name(Args): a function of another package, declared in Externs
//	extern_method X.Name(Args): a method of the type TypeName of Path, declared in Externs
//	binop    X Op Y
//	unop     Op X     ("!", "-", "*" deref, "&" addr)
//	struct   Name{Fields}  (Args[i] value of FieldNames[i])
//	conv     Name(X): conversion to a named type
//	zero     T
//	make     T: an empty map or slice
//	intrinsic Name(Args): sprintf, errorf, println, append, len
//	to_error X: a value of an error struct used as error
//	func_ref Name, X: a module function, or with X the method Name bound to X, as a value
//	lambda   Params Results Captures Body: a function literal
//	call_value X(Args): a call of the function value X
//	spread   X...: a slice passed as the last argument of a call of a variadic function or append
//	iface_call X.Name(Args): a call through X, a value of the interface Iface of Path ("": this package)
//	to_iface X: a value used where a value of the interface T is expected
//
// Case is one clause of a switch; no Values for the default clause.
type Case struct {
	Values []Expr `json:"values"`
	Body   []Stmt `json:"body"`
}

type Expr struct {
	K          string   `json:"k"`
	T          *Type    `json:"t,omitempty"`
	Name       string   `json:"name,omitempty"`
	Path       string   `json:"path,omitempty"`
	TypeName   string   `json:"type,omitempty"`  // extern_method: the receiver type's name
	Iface      string   `json:"iface,omitempty"` // iface_call: the interface's name
	Value      string   `json:"value,omitempty"`
	Op         string   `json:"op,omitempty"`
	X          *Expr    `json:"x,omitempty"`
	Y          *Expr    `json:"y,omitempty"`
	Args       []Expr   `json:"args,omitempty"`
	FieldNames []string `json:"field_names,omitempty"`
	// call, extern, extern_method: the type arguments of a call of a generic function
	Targs []Type `json:"targs,omitempty"`
	// lambda: its parameters, results, the outer names it uses, and its body.
	Params   []Param  `json:"params,omitempty"`
	Results  []Type   `json:"results,omitempty"`
	Captures []string `json:"captures,omitempty"`
	Body     []Stmt   `json:"body,omitempty"`
}

type Refusal struct {
	Pos    string `json:"pos"`
	What   string `json:"what"`
	Reason string `json:"reason"`
}

// TypeParam is a type parameter of a generic function, type or extern, with its constraint: any,
// or an interface type.
type TypeParam struct {
	Name       string `json:"name"`
	Constraint Type   `json:"constraint"`
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
