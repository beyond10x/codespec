package main

import "encoding/json"

// The cir/1 module as this writer decodes it. cir/1 is defined by the Rust crate
// b10x-codespec-model; each Go struct here holds the union of the fields one tagged node kind can
// carry, and the codespec CLI has validated the document before it reaches this program.

type Module struct {
	Format  string     `json:"format"`
	Name    string     `json:"name"`
	Reader  Reader     `json:"reader"`
	Doc     string     `json:"doc"`
	Types   []TypeDecl `json:"types"`
	Consts  []Const    `json:"consts"`
	Errors  []ErrorVar `json:"errors"`
	Vars    []Var      `json:"vars"`
	Funcs   []Func     `json:"funcs"`
	Externs any        `json:"externs"`
	Layout  []string   `json:"layout"`
	Facts   any        `json:"facts"`
	Refuse  any        `json:"refusals"`
	// Interfaces: the module's own (Path "") and those of other packages a call goes through.
	Interfaces []Interface `json:"interfaces"`
}

type Reader struct {
	Language string `json:"language"`
	Name     string `json:"name"`
	Version  string `json:"version"`
}

type Type struct {
	K      string `json:"k"`
	Width  string `json:"width"`
	Signed bool   `json:"signed"`
	Name   string `json:"name"`
	Path   string `json:"path"`
	Elem   *Type  `json:"elem"`
	Key    *Type  `json:"key"`
	Dir    string `json:"dir"`
	Elems  []Type `json:"elems"`
	Bits   string `json:"bits"` // float
	Len    int64  `json:"len"`  // array
	// func
	Params  []Type `json:"params"`
	Results []Type `json:"results"`
	// named, extern: the type arguments of an instantiated generic type
	Targs []Type `json:"targs"`
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

type TypeDecl struct {
	K        string          `json:"k"`
	Name     string          `json:"name"`
	Exported bool            `json:"exported"`
	Fields   []Field         `json:"fields"`
	Under    *Type           `json:"under"`
	Variants json.RawMessage `json:"variants"`
	Doc      string          `json:"doc"`
	Pos      string          `json:"pos"`
	Tparams  []TypeParam     `json:"tparams"`
}

type Const struct {
	Name  string `json:"name"`
	T     Type   `json:"t"`
	Value Expr   `json:"value"`
	Pos   string `json:"pos"`
}

type ErrorVar struct {
	Name    string `json:"name"`
	Message string `json:"message"`
	Pos     string `json:"pos"`
}

// Var is a module-level variable: its zero value, Init, or an initializer kept as Foreign source.
type Var struct {
	Name    string       `json:"name"`
	T       Type         `json:"t"`
	Init    *Expr        `json:"init"`
	Foreign *ForeignInit `json:"foreign"`
	Pos     string       `json:"pos"`
}

type ForeignInit struct {
	Lang   string `json:"lang"`
	Src    string `json:"src"`
	Reason string `json:"reason"`
}

type Param struct {
	Name     string `json:"name"`
	T        Type   `json:"t"`
	Variadic bool   `json:"variadic"`
}

type Func struct {
	Name     string      `json:"name"`
	Exported bool        `json:"exported"`
	Tparams  []TypeParam `json:"tparams"`
	Recv     *Param      `json:"recv"`
	Params   []Param     `json:"params"`
	Results  []Type      `json:"results"`
	Body     []Stmt      `json:"body"`
	Doc      string      `json:"doc"`
	Pos      string      `json:"pos"`
}

func (f Func) key() string {
	if f.Recv == nil {
		return f.Name
	}
	return recvTypeName(f.Recv.T) + "." + f.Name
}

func recvTypeName(t Type) string {
	if t.K == "ptr" && t.Elem != nil {
		return recvTypeName(*t.Elem)
	}
	if t.K == "named" {
		return t.Name
	}
	return ""
}

type Case struct {
	Values []Expr `json:"values"`
	Body   []Stmt `json:"body"`
}

type Stmt struct {
	K       string   `json:"k"`
	Names   []string `json:"names"`
	Name    string   `json:"name"`
	T       *Type    `json:"t"`
	Op      string   `json:"op"`
	Lhs     []Expr   `json:"lhs"`
	Rhs     []Expr   `json:"rhs"`
	X       *Expr    `json:"x"`
	Cond    *Expr    `json:"cond"`
	Then    []Stmt   `json:"then"`
	Else    []Stmt   `json:"else"`
	Key     string   `json:"key"`
	Val     string   `json:"val"`
	Over    *Expr    `json:"over"`
	Body    []Stmt   `json:"body"`
	Init    *Stmt    `json:"init"`
	Post    *Stmt    `json:"post"`
	Tag     *Expr    `json:"tag"`
	Cases   []Case   `json:"cases"`
	Results []Expr   `json:"results"`
	Chan    *Expr    `json:"chan"`
	Map     *Expr    `json:"map"`
	Index   *Expr    `json:"index"`
	Lang    string   `json:"lang"`
	Src     string   `json:"src"`
	Reason  string   `json:"reason"`
	Pos     string   `json:"pos"`
}

type FieldInit struct {
	Name  string `json:"name"`
	Value Expr   `json:"value"`
}

type Expr struct {
	K      string      `json:"k"`
	T      *Type       `json:"t"`
	Name   string      `json:"name"`
	Path   string      `json:"path"`
	Type   string      `json:"type"`
	Iface  string      `json:"iface"` // iface_call
	Targs  []Type      `json:"targs"` // call, extern, extern_method: type arguments
	Value  string      `json:"value"`
	Op     string      `json:"op"`
	X      *Expr       `json:"x"`
	Y      *Expr       `json:"y"`
	Index  *Expr       `json:"index"`
	Args   []Expr      `json:"args"`
	Fields []FieldInit `json:"fields"`
	// func_ref: the bound receiver; call_value: the function value called.
	Recv *Expr `json:"recv"`
	F    *Expr `json:"f"`
	// lambda
	Params   []Param  `json:"params"`
	Results  []Type   `json:"results"`
	Captures []string `json:"captures"`
	Body     []Stmt   `json:"body"`
}

// Interface is an interface type with its methods; with Path, one of another package.
type Interface struct {
	Path     string        `json:"path"`
	Name     string        `json:"name"`
	Exported bool          `json:"exported"`
	Methods  []IfaceMethod `json:"methods"`
	Doc      string        `json:"doc"`
	Pos      string        `json:"pos"`
}

type IfaceMethod struct {
	Name    string `json:"name"`
	Params  []Type `json:"params"`
	Results []Type `json:"results"`
}

// TypeParam is a type parameter of a generic function or type with its constraint, an interface.
type TypeParam struct {
	Name       string `json:"name"`
	Constraint Type   `json:"constraint"`
}
