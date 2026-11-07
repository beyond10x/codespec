package main

import (
	"bytes"
	"encoding/json"
	"fmt"
	"go/ast"
	"go/build"
	"go/importer"
	"go/parser"
	"go/token"
	"go/types"
	"os"
	"os/exec"
	"sort"
	"strconv"
	"strings"
)

// chanCapacity is the room of each channel passed to a call, as in crates/conformance.
const chanCapacity = 64

// Call is one call of a codespec-calls/1 document.
type Call struct {
	Func string   `json:"func"`
	Args []string `json:"args"`
}

// Calls is a codespec-calls/1 document.
type Calls struct {
	Format string `json:"format"`
	Calls  []Call `json:"calls"`
}

// readCalls reads a codespec-calls/1 document as read_calls in crates/conformance does: unknown
// fields, another format, a call whose args are null, a list of no calls and any encoding other than
// the canonical one (two-space indentation, a final newline; so also a key in other case, a key
// stated twice and anything after the document) are refused.
func readCalls(path string) (*Calls, error) {
	b, err := os.ReadFile(path)
	if err != nil {
		return nil, err
	}
	dec := json.NewDecoder(bytes.NewReader(b))
	dec.DisallowUnknownFields()
	var c Calls
	if err := dec.Decode(&c); err != nil {
		return nil, fmt.Errorf("%s: %w", path, err)
	}
	if c.Format != "codespec-calls/1" {
		return nil, fmt.Errorf("%s: format is %q, this runner reads \"codespec-calls/1\"", path, c.Format)
	}
	if len(c.Calls) == 0 {
		return nil, fmt.Errorf("%s: no calls", path)
	}
	for _, call := range c.Calls {
		if call.Args == nil {
			return nil, fmt.Errorf("%s: %s: args is not a list", path, call.Func)
		}
	}
	var canon bytes.Buffer
	enc := json.NewEncoder(&canon)
	enc.SetEscapeHTML(false)
	enc.SetIndent("", "  ")
	if err := enc.Encode(c); err != nil {
		return nil, fmt.Errorf("%s: %w", path, err)
	}
	if !bytes.Equal(canon.Bytes(), b) {
		return nil, fmt.Errorf("%s: not canonical (two-space indentation, a final newline)", path)
	}
	return &c, nil
}

// load parses and type-checks the one package in dir, ignoring tests, and returns it with its
// files, sorted.
func load(dir string) (*types.Package, []string, error) {
	fset := token.NewFileSet()
	pkgs, err := parser.ParseDir(fset, dir, func(fi os.FileInfo) bool {
		return !strings.HasSuffix(fi.Name(), "_test.go")
	}, 0)
	if err != nil {
		return nil, nil, err
	}
	if len(pkgs) != 1 {
		return nil, nil, fmt.Errorf("%s: want exactly one package, found %d", dir, len(pkgs))
	}
	var name string
	var names []string
	var files []*ast.File
	for n, p := range pkgs {
		name = n
		for fn := range p.Files {
			names = append(names, fn)
		}
		sort.Strings(names)
		for _, fn := range names {
			files = append(files, p.Files[fn])
		}
	}
	ensureGOROOT()
	conf := types.Config{Importer: importer.ForCompiler(fset, "source", nil)}
	pkg, err := conf.Check(name, fset, files, nil)
	if err != nil {
		return nil, nil, err
	}
	return pkg, names, nil
}

// ensureGOROOT: a binary built with -trimpath carries no GOROOT, and the source importer then
// cannot find the standard library. Ask the installed Go toolchain where it is.
func ensureGOROOT() {
	if build.Default.GOROOT != "" {
		return
	}
	out, err := exec.Command("go", "env", "GOROOT").Output()
	if err != nil {
		return
	}
	build.Default.GOROOT = strings.TrimSpace(string(out))
}

func isError(t types.Type) bool {
	return types.Identical(t, types.Universe.Lookup("error").Type())
}

// printable refuses a result or channel element type the driver cannot print.
func printable(t types.Type) error {
	switch u := types.Unalias(t).(type) {
	case *types.Basic:
		if u.Info()&types.IsUnsigned != 0 {
			return fmt.Errorf("unsigned %s cannot be printed: the driver prints signed integers only", t)
		}
		if u.Info()&(types.IsInteger|types.IsBoolean|types.IsString) != 0 {
			return nil
		}
	case *types.Slice:
		return printable(u.Elem())
	case *types.Interface:
		if u.Empty() {
			return nil
		}
	case *types.Named:
		under := t.Underlying()
		switch under.(type) {
		case *types.Basic, *types.Slice:
			return printable(under)
		}
		return fmt.Errorf("result type %s is not a newtype or enum", t)
	}
	return fmt.Errorf("result type %s cannot be printed", t)
}

// argKind is the kind of parameter an argument is given to: int, bool, string or chan.
func argKind(t types.Type) (string, error) {
	switch u := types.Unalias(t).(type) {
	case *types.Basic:
		switch {
		case u.Info()&types.IsInteger != 0:
			return "int", nil
		case u.Info()&types.IsBoolean != 0:
			return "bool", nil
		case u.Info()&types.IsString != 0:
			return "string", nil
		}
	case *types.Chan:
		if u.Dir() == types.RecvOnly {
			return "", fmt.Errorf("a receive-only channel parameter cannot be passed")
		}
		return "chan", nil
	}
	return "", fmt.Errorf("parameter type %s cannot be passed", t)
}

// checkArgument is check_argument of crates/conformance: whether arg is a value of kind.
func checkArgument(arg, kind string) error {
	ok := false
	switch kind {
	case "int":
		digits := strings.TrimPrefix(arg, "-")
		_, err := strconv.ParseInt(arg, 10, 64)
		ok = digits != "" && strings.Trim(digits, "0123456789") == "" &&
			(digits == "0" || !strings.HasPrefix(digits, "0")) && arg != "-0" && err == nil
	case "bool":
		ok = arg == "true" || arg == "false"
	case "string":
		ok = len(arg) >= 2 && arg[0] == '"' && arg[len(arg)-1] == '"'
		for i := 1; ok && i < len(arg)-1; i++ {
			b := arg[i]
			ok = b >= ' ' && b <= '~' && b != '"' && b != '\\'
		}
	case "chan":
		if arg != "chan" {
			return fmt.Errorf("argument %q for a channel parameter is not `chan`", arg)
		}
		ok = true
	}
	if !ok {
		return fmt.Errorf("argument %q is not a %s value", arg, kind)
	}
	return nil
}

// callsGo is calls_go of crates/conformance, with each function's signature taken from the
// type-checked package: the driver's calls.go, func run() making each call in its own block.
func callsGo(pkg *types.Package, calls *Calls) (string, error) {
	qual := func(p *types.Package) string {
		if p == pkg {
			return "subject"
		}
		return p.Name()
	}
	var s strings.Builder
	s.WriteString("// Code generated from calls.json. DO NOT EDIT.\n\npackage main\n\n")
	s.WriteString("import subject \"codespec.conformance/case/subject\"\n\nfunc run() {\n")
	for _, c := range calls.Calls {
		obj, _ := pkg.Scope().Lookup(c.Func).(*types.Func)
		if obj == nil || !obj.Exported() {
			return "", fmt.Errorf("no exported function %q", c.Func)
		}
		sig := obj.Type().(*types.Signature)
		if sig.Params().Len() != len(c.Args) {
			return "", fmt.Errorf("%s takes %d arguments, the call passes %d", c.Func, sig.Params().Len(), len(c.Args))
		}
		var args, chans []string
		s.WriteString("\t{\n")
		for i, a := range c.Args {
			t := sig.Params().At(i).Type()
			kind, err := argKind(t)
			if err != nil {
				return "", fmt.Errorf("%s: %w", c.Func, err)
			}
			if err := checkArgument(a, kind); err != nil {
				return "", fmt.Errorf("%s: %w", c.Func, err)
			}
			if kind != "chan" {
				args = append(args, a)
				continue
			}
			elem := t.(*types.Chan).Elem()
			if err := printable(elem); err != nil {
				return "", fmt.Errorf("%s: %w", c.Func, err)
			}
			name := fmt.Sprintf("ch%d", i)
			fmt.Fprintf(&s, "\t\t%s := make(chan %s, %d)\n", name, types.TypeString(elem, qual), chanCapacity)
			args = append(args, name)
			chans = append(chans, name)
		}
		how, err := shape(sig.Results())
		if err != nil {
			return "", fmt.Errorf("%s: %w", c.Func, err)
		}
		joined := strings.Join(args, ", ")
		if how == "none" {
			fmt.Fprintf(&s, "\t\tsubject.%s(%s)\n\t\tnone()\n", c.Func, joined)
		} else {
			fmt.Fprintf(&s, "\t\t%s(subject.%s(%s))\n", how, c.Func, joined)
		}
		for _, ch := range chans {
			fmt.Fprintf(&s, "\t\tdrain(%s)\n", ch)
		}
		s.WriteString("\t}\n")
	}
	s.WriteString("}\n")
	return s.String(), nil
}

// shape is how the driver prints a function's results: none, values or fallible.
func shape(results *types.Tuple) (string, error) {
	n := results.Len()
	if n == 0 {
		return "none", nil
	}
	values, how := n, "values"
	if isError(results.At(n - 1).Type()) {
		values, how = n-1, "fallible"
	}
	for i := 0; i < values; i++ {
		if err := printable(results.At(i).Type()); err != nil {
			return "", err
		}
	}
	return how, nil
}
