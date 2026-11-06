package main

import (
	"bytes"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"testing"
)

// besideAPackageFunc is what codespec-read-java reads, with 0 refusals, from
//
//	static int <name>(int n) { return n; }
//	public String keys(TreeMap<String, Integer> m) { String out = ""; for (String id : m.keySet()) out += id; return out; }
//
// a package-private static method (a free, unexported Go function of that name) beside a loop
// over a TreeMap's keys (checked: probe package with `static int make(int n)`, read with no
// refusal).
func besideAPackageFunc(name string) string {
	m := jv("m", jmap(tStr, tI32))
	keys := fmt.Sprintf(`{"k": "intrinsic", "name": "sorted_keys", "args": [%s], "t": %s}`, m, jslice(tStr))
	return fmt.Sprintf(`{"format": "cir/1", "name": "builtin",
      "reader": {"language": "java", "name": "test", "version": "0"},
      "types": [],
      "funcs": [
        {"name": %[1]q, "exported": false, "pos": "S.java:1",
          "params": [{"name": "n", "t": %[2]s}], "results": [%[2]s],
          "body": [%[3]s]},
        {"name": "Keys", "exported": true, "pos": "S.java:2",
          "params": [{"name": "m", "t": %[4]s}], "results": [%[5]s],
          "body": [
            {"k": "let", "names": ["out"], "rhs": [%[6]s], "pos": "S.java:3"},
            {"k": "for_range", "key": "_", "val": "id", "over": %[7]s, "pos": "S.java:4", "body": [
              {"k": "assign", "op": "+=", "lhs": [%[8]s], "rhs": [%[9]s], "pos": "S.java:5"}]},
            %[10]s]}],
      "layout": ["func:%[1]s", "func:Keys"]}`,
		name, tI32, jret("S.java:1", jv("n", tI32)), jmap(tStr, tI32), tStr, jlit(`""`, tStr), keys,
		jv("out", tStr), jv("id", tStr), jret("S.java:6", jv("out", tStr)))
}

// writtenOrRefused runs the writer on doc and, when it writes, requires `go vet` to pass.
func writtenOrRefused(t *testing.T, doc string) {
	t.Helper()
	out := filepath.Join(scratch(t), "out")
	if err := run(strings.NewReader(doc), out); err != nil {
		return // a refusal is an acceptable answer
	}
	files, _ := filepath.Glob(filepath.Join(out, "*.go"))
	cmd := exec.Command("go", append([]string{"vet"}, files...)...)
	var stderr bytes.Buffer
	cmd.Stderr = &stderr
	if err := cmd.Run(); err != nil {
		src, _ := os.ReadFile(files[0])
		t.Fatalf("written without a refusal, and the Go does not compile: %v\n%s\n%s", err, stderr.String(), src)
	}
}

// The sorted_keys closure calls make, len and append. shadowedIn refuses only a local of the
// function being written that holds one of those names; a package-level function of that name
// (a Java static method `make`, `len` or `append`, all legal Java) shadows the predeclared one
// for the whole package, and the closure does not compile. The base commit refused sorted_keys
// outright, so this Go is new.
func TestAdversary2SortedKeysBesideAPackageFunctionNamedLikeABuiltin(t *testing.T) {
	for _, name := range []string{"make", "len", "append"} {
		t.Run(name, func(t *testing.T) {
			writtenOrRefused(t, besideAPackageFunc(name))
		})
	}
}

// A Java int parameter named fmt (legal Java, read with 0 refusals) beside String.format: the
// writer prints fmt.Sprintf under the plain import name, which the parameter shadows. The
// writer's own helpers are imported under fresh aliases; its fmt and errors are not.
// Pre-existing: the base commit's writer writes the same Go.
func TestAdversary2SprintfBesideALocalNamedFmt(t *testing.T) {
	doc := fmt.Sprintf(`{"format": "cir/1", "name": "label",
      "reader": {"language": "java", "name": "test", "version": "0"},
      "types": [],
      "funcs": [{"name": "Label", "exported": true, "pos": "B.java:8",
        "params": [{"name": "fmt", "t": %[1]s}], "results": [%[2]s],
        "body": [%[3]s]}],
      "layout": ["func:Label"]}`,
		tI32, tStr, jret("B.java:9", jsprintf(tStr, jlit(`"%d"`, tStr), jv("fmt", tI32))))
	writtenOrRefused(t, doc)
}
