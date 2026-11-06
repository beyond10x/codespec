package main

import (
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"testing"
)

// The forms the Java reader emits for the ledger example: sorted_keys, a conversion to a built-in
// integer type, and a field and a method of one name on one type.

const (
	tStr   = `{"k": "string"}`
	tI32   = `{"k": "int", "width": "32", "signed": true}`
	tI64   = `{"k": "int", "width": "64", "signed": true}`
	tIntP  = `{"k": "int", "width": "platform", "signed": true}`
	tAcct  = `{"k": "named", "name": "Acct"}`
	tPAcct = `{"k": "ptr", "elem": {"k": "named", "name": "Acct"}}`
)

func jv(name, t string) string { return fmt.Sprintf(`{"k": "var", "name": %q, "t": %s}`, name, t) }
func jlit(v, t string) string  { return fmt.Sprintf(`{"k": "lit", "value": %q, "t": %s}`, v, t) }
func jmap(k, e string) string  { return fmt.Sprintf(`{"k": "map", "key": %s, "elem": %s}`, k, e) }
func jslice(e string) string   { return fmt.Sprintf(`{"k": "slice", "elem": %s}`, e) }
func jret(pos string, xs ...string) string {
	return fmt.Sprintf(`{"k": "return", "pos": %q, "results": [%s]}`, pos, strings.Join(xs, ", "))
}
func jfield(x, name, t string) string {
	return fmt.Sprintf(`{"k": "field", "x": %s, "name": %q, "t": %s}`, x, name, t)
}
func jsprintf(t string, args ...string) string {
	return fmt.Sprintf(`{"k": "intrinsic", "name": "sprintf", "args": [%s], "t": %s}`, strings.Join(args, ", "), t)
}

// joinKeys is `func <name>(m map[K]int32) string`, appending sprintf(format, k, m[k]) for each k
// of sorted_keys(m), as the Java reader reads a loop over a TreeMap's keySet.
func joinKeys(name, key, format string) string {
	m := jv("m", jmap(key, tI32))
	keys := fmt.Sprintf(`{"k": "intrinsic", "name": "sorted_keys", "args": [%s], "t": %s}`, m, jslice(key))
	idx := fmt.Sprintf(`{"k": "index", "x": %s, "index": %s, "t": %s}`, m, jv("k", key), tI32)
	return fmt.Sprintf(`{"name": %q, "exported": true, "pos": "K.java:1",
      "params": [{"name": "m", "t": %s}], "results": [%s],
      "body": [
        {"k": "let", "names": ["out"], "rhs": [%s], "pos": "K.java:2"},
        {"k": "for_range", "key": "_", "val": "k", "over": %s, "pos": "K.java:3", "body": [
          {"k": "assign", "op": "+=", "lhs": [%s], "rhs": [%s], "pos": "K.java:4"}]},
        %s]}`,
		name, jmap(key, tI32), tStr, jlit(`""`, tStr), keys, jv("out", tStr),
		jsprintf(tStr, jlit(format, tStr), jv("k", key), idx), jret("K.java:5", jv("out", tStr)))
}

// keysDoc holds Join over string keys, Nums over int64 keys and Count, int32(len(xs)).
func keysDoc() string {
	count := fmt.Sprintf(`{"name": "Count", "exported": true, "pos": "K.java:9",
      "params": [{"name": "xs", "t": %s}], "results": [%s],
      "body": [%s]}`, jslice(tI64), tI32,
		jret("K.java:10", fmt.Sprintf(`{"k": "conv", "name": "int32", "x": {"k": "intrinsic", "name": "len", "args": [%s], "t": %s}, "t": %s}`,
			jv("xs", jslice(tI64)), tIntP, tI32)))
	return fmt.Sprintf(`{"format": "cir/1", "name": "keys",
      "reader": {"language": "java", "name": "test", "version": "0"},
      "types": [], "funcs": [%s, %s, %s], "layout": ["func:Join", "func:Nums", "func:Count"]}`,
		joinKeys("Join", tStr, `"%s=%d;"`), joinKeys("Nums", tI64, `"%d=%d;"`), count)
}

// runWith builds the written package beside a main package holding mainSrc and runs it.
func runWith(t *testing.T, out, pkg, mainSrc string) string {
	t.Helper()
	root := filepath.Dir(out)
	if err := os.WriteFile(filepath.Join(root, "go.mod"), []byte("module probe\n\ngo 1.23\n"), 0o644); err != nil {
		t.Fatal(err)
	}
	if err := os.MkdirAll(filepath.Join(root, "main"), 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(root, "main", "main.go"), []byte(mainSrc), 0o644); err != nil {
		t.Fatal(err)
	}
	cmd := exec.Command("go", "run", "./main")
	cmd.Dir = root
	got, err := cmd.CombinedOutput()
	if err != nil {
		t.Fatalf("go run: %v\n%s", err, got)
	}
	return string(got)
}

// writeInto writes doc into <scratch>/<pkg>, the layout runWith expects.
func writeInto(t *testing.T, doc, pkg string) (string, string) {
	t.Helper()
	out := filepath.Join(scratch(t), pkg)
	if err := run(strings.NewReader(doc), out); err != nil {
		t.Fatalf("write: %v", err)
	}
	src, err := os.ReadFile(filepath.Join(out, pkg+".go"))
	if err != nil {
		t.Fatal(err)
	}
	return out, string(src)
}

func TestSortedKeysOrdersStringsByUTF16CodeUnitAndIntegersNumerically(t *testing.T) {
	out, src := writeInto(t, keysDoc(), "keys")
	vet(t, out)
	got := runWith(t, out, "keys", `package main

import (
	"fmt"

	"probe/keys"
)

func main() {
	fmt.Println(keys.Join(map[string]int32{"ｚ": 1, "𝄞": 2, "b": 3, "Ω": 4, "": 5, "B": 6}))
	fmt.Println(keys.Nums(map[int64]int32{20: 1, -5: 2, 3: 3, -100: 4, 9223372036854775807: 5, -9223372036854775808: 6}))
}
`)
	// Java's String.compareTo: U+1D11E is the surrogate pair D834 DD1E, before U+FF5A; UTF-8 byte
	// order would put it last.
	want := "=5;B=6;b=3;Ω=4;𝄞=2;ｚ=1;\n-9223372036854775808=6;-100=4;-5=2;3=3;20=1;9223372036854775807=5;\n"
	if got != want {
		t.Fatalf("sorted_keys order:\n got %q\nwant %q\n%s", got, want, src)
	}
}

func TestConvToABuiltInIntegerTypeIsAGoConversion(t *testing.T) {
	out, src := writeInto(t, keysDoc(), "keys")
	vet(t, out)
	if !strings.Contains(src, "return int32(len(xs))") {
		t.Fatalf("conv to int32 is not int32(len(xs)):\n%s", src)
	}
}

// acctDoc is a Java-shaped class: fields id and balance, a constructor NewAcct through a local
// this, an exported method id (Java's public getter) and an unexported method balance, each named
// as a field is.
func acctDoc() string {
	this := jv("this", tPAcct)
	return fmt.Sprintf(`{"format": "cir/1", "name": "acct",
      "reader": {"language": "java", "name": "test", "version": "0"},
      "types": [{"k": "struct", "name": "Acct", "exported": true, "pos": "Acct.java:1",
        "fields": [{"name": "id", "t": %[1]s}, {"name": "balance", "t": %[2]s}]}],
      "funcs": [
        {"name": "NewAcct", "exported": true, "pos": "Acct.java:2",
         "params": [{"name": "id", "t": %[1]s}, {"name": "b", "t": %[2]s}], "results": [%[3]s],
         "body": [
           {"k": "let", "names": ["this"], "rhs": [{"k": "unop", "op": "&", "x": {"k": "struct", "name": "Acct", "fields": [], "t": %[4]s}, "t": %[3]s}], "pos": "Acct.java:2"},
           {"k": "assign", "lhs": [%[5]s], "rhs": [%[6]s], "pos": "Acct.java:3"},
           {"k": "assign", "lhs": [%[7]s], "rhs": [%[8]s], "pos": "Acct.java:4"},
           %[9]s]},
        {"name": "id", "exported": true, "recv": {"name": "this", "t": %[3]s}, "pos": "Acct.java:5",
         "params": [], "results": [%[1]s], "body": [%[10]s]},
        {"name": "balance", "exported": false, "recv": {"name": "this", "t": %[3]s}, "pos": "Acct.java:6",
         "params": [], "results": [%[2]s], "body": [%[11]s]},
        {"name": "show", "exported": true, "recv": {"name": "this", "t": %[3]s}, "pos": "Acct.java:7",
         "params": [], "results": [%[1]s], "body": [%[12]s]}],
      "layout": ["type:Acct", "func:NewAcct", "func:Acct.id", "func:Acct.balance", "func:Acct.show"]}`,
		tStr, tI64, tPAcct, tAcct,
		jfield(this, "id", tStr), jv("id", tStr),
		jfield(this, "balance", tI64), jv("b", tI64),
		jret("Acct.java:2", this),
		jret("Acct.java:5", jfield(this, "id", tStr)),
		jret("Acct.java:6", jfield(this, "balance", tI64)),
		jret("Acct.java:7", jsprintf(tStr, jlit(`"%s has %d"`, tStr), jfield(this, "id", tStr),
			fmt.Sprintf(`{"k": "method", "x": %s, "name": "balance", "args": [], "t": %s}`, this, tI64))))
}

func TestAFieldAndAMethodOfOneNameAreBothWritten(t *testing.T) {
	out, src := writeInto(t, acctDoc(), "acct")
	vet(t, out)
	for _, want := range []string{
		"\tid       string\n",
		"\tbalance_ int64\n",
		"func (this *Acct) Id() string {\n\treturn this.id\n}",
		"func (this *Acct) balance() int64 {\n\treturn this.balance_\n}",
		"this.balance_ = b",
	} {
		if !strings.Contains(src, want) {
			t.Errorf("want %q in:\n%s", want, src)
		}
	}
	got := runWith(t, out, "acct", `package main

import (
	"fmt"

	"probe/acct"
)

func main() {
	a := acct.NewAcct("ω", -7)
	fmt.Println(a.Id())
	fmt.Println(a.Show())
}
`)
	if want := "ω\nω has -7\n"; got != want {
		t.Fatalf("got %q, want %q", got, want)
	}
}

// An exported name that Go would read as unexported is spelled with an upper-case first letter
// wherever it is declared or used.
func TestAnExportedLowerCaseNameIsWrittenExported(t *testing.T) {
	_, src := writeInto(t, acctDoc(), "acct")
	if !strings.Contains(src, "func (this *Acct) Show() string") || strings.Contains(src, ") show()") {
		t.Fatalf("exported method show not written as Show:\n%s", src)
	}
}

// A Java static method `len` and a constant `append` (both legal Java) are package-level Go
// names that would hide the predeclared ones from the whole package, sorted_keys' closure
// included. Each is renamed with `_`, every reference follows, and the package builds and runs.
func TestAPackageLevelNameLikeAPredeclaredOneIsRenamed(t *testing.T) {
	m := jv("m", jmap(tStr, tI32))
	keys := fmt.Sprintf(`{"k": "intrinsic", "name": "sorted_keys", "args": [%s], "t": %s}`, m, jslice(tStr))
	call := fmt.Sprintf(`{"k": "call", "name": "len", "args": [{"k": "const", "name": "append", "t": %s}], "t": %s}`, tI32, tI32)
	doc := fmt.Sprintf(`{"format": "cir/1", "name": "pre",
      "reader": {"language": "java", "name": "test", "version": "0"},
      "types": [],
      "consts": [{"name": "append", "t": %[1]s, "value": %[2]s, "pos": "P.java:1"}],
      "funcs": [
        {"name": "len", "exported": false, "pos": "P.java:2",
         "params": [{"name": "n", "t": %[1]s}], "results": [%[1]s], "body": [%[3]s]},
        {"name": "Keys", "exported": true, "pos": "P.java:3",
         "params": [{"name": "m", "t": %[4]s}], "results": [%[5]s],
         "body": [
           {"k": "let", "names": ["out"], "rhs": [%[6]s], "pos": "P.java:4"},
           {"k": "for_range", "key": "_", "val": "id", "over": %[7]s, "pos": "P.java:5", "body": [
             {"k": "assign", "op": "+=", "lhs": [%[8]s], "rhs": [%[9]s], "pos": "P.java:6"}]},
           %[10]s]}],
      "layout": ["const:append", "func:len", "func:Keys"]}`,
		tI32, jlit("7", tI32), jret("P.java:2", jv("n", tI32)), jmap(tStr, tI32), tStr, jlit(`""`, tStr), keys,
		jv("out", tStr), jsprintf(tStr, jlit(`"%s%d;"`, tStr), jv("id", tStr), call), jret("P.java:7", jv("out", tStr)))
	out, src := writeInto(t, doc, "pre")
	vet(t, out)
	for _, want := range []string{"func len_(n int32) int32", "append_ int32 = 7", "len_(append_)"} {
		if !strings.Contains(src, want) {
			t.Errorf("want %q in:\n%s", want, src)
		}
	}
	got := runWith(t, out, "pre", "package main\n\nimport (\n\t\"fmt\"\n\n\t\"probe/pre\"\n)\n\nfunc main() { fmt.Println(pre.Keys(map[string]int32{\"b\": 1, \"a\": 2})) }\n")
	if got != "a7;b7;\n" {
		t.Fatalf("got %q", got)
	}
}

// A parameter named fmt beside sprintf: fmt is imported under the alias fmt_, and sprintf is
// written through it; without a clash the import stays the plain "fmt".
func TestSprintfBesideALocalNamedFmtUsesAnAlias(t *testing.T) {
	doc := func(param string) string {
		return fmt.Sprintf(`{"format": "cir/1", "name": "label",
      "reader": {"language": "java", "name": "test", "version": "0"},
      "types": [],
      "funcs": [{"name": "Label", "exported": true, "pos": "B.java:8",
        "params": [{"name": %[4]q, "t": %[1]s}], "results": [%[2]s],
        "body": [%[3]s]}],
      "layout": ["func:Label"]}`,
			tI32, tStr, jret("B.java:9", jsprintf(tStr, jlit(`"%d"`, tStr), jv(param, tI32))), param)
	}
	out, src := writeInto(t, doc("fmt"), "label")
	vet(t, out)
	if !strings.Contains(src, `fmt_ "fmt"`) || !strings.Contains(src, "fmt_.Sprintf(\"%d\", fmt)") {
		t.Fatalf("sprintf beside a local fmt not written through an alias:\n%s", src)
	}
	_, src = writeInto(t, doc("n"), "label")
	if !strings.Contains(src, "import \"fmt\"\n") {
		t.Fatalf("without a clash, fmt is not the plain import:\n%s", src)
	}
}

// The class behind the adversary's sort/utf16/string cases: every name the sorted_keys literal
// takes from outside it. A local holding a package name is answered by the alias; a local holding
// a predeclared name the literal uses refuses the function at its position; either way nothing
// written fails to compile.
func TestSortedKeysBesideALocalNamedLikeAnythingTheLiteralUses(t *testing.T) {
	for _, local := range []string{"sort", "utf16", "sort_", "string", "int32", "make", "len", "append", "rune", "int", "bool"} {
		t.Run(local, func(t *testing.T) {
			out := filepath.Join(scratch(t), "shadow")
			err := run(strings.NewReader(shadowedSortedKeys(local)), out)
			switch local {
			case "sort", "utf16", "sort_":
				if err != nil {
					t.Fatalf("a local named like a package is answered by the alias, not refused: %v", err)
				}
				vet(t, out)
			default:
				if err == nil || !strings.Contains(err.Error(), "S.java:3") || !strings.Contains(err.Error(), local) {
					t.Fatalf("want a refusal at S.java:3 naming %s, got %v", local, err)
				}
			}
		})
	}
}
