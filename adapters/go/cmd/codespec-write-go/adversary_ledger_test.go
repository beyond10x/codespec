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

// shadowedSortedKeys is what codespec-read-java reads from
//
//	public String keys(int <local>) { String out = ""; for (String id : numbers.keySet()) out += id; return out; }
//
// over a TreeMap field: a sorted_keys loop in a function that also binds a local whose name is
// one of the packages the written sort closure refers to.
func shadowedSortedKeys(local string) string {
	m := jv("m", jmap(tStr, tI32))
	keys := fmt.Sprintf(`{"k": "intrinsic", "name": "sorted_keys", "args": [%s], "t": %s}`, m, jslice(tStr))
	return fmt.Sprintf(`{"format": "cir/1", "name": "shadow",
      "reader": {"language": "java", "name": "test", "version": "0"},
      "types": [],
      "funcs": [{"name": "Keys", "exported": true, "pos": "S.java:1",
        "params": [{"name": "m", "t": %[1]s}, {"name": %[2]q, "t": %[3]s}], "results": [%[4]s],
        "body": [
          {"k": "let", "names": ["out"], "rhs": [%[5]s], "pos": "S.java:2"},
          {"k": "for_range", "key": "_", "val": "id", "over": %[6]s, "pos": "S.java:3", "body": [
            {"k": "assign", "op": "+=", "lhs": [%[7]s], "rhs": [%[8]s], "pos": "S.java:4"}]},
          %[9]s]}],
      "layout": ["func:Keys"]}`,
		jmap(tStr, tI32), local, tI32, tStr, jlit(`""`, tStr), keys, jv("out", tStr), jv("id", tStr),
		jret("S.java:5", jv("out", tStr)))
}

// A Java parameter or local named sort, utf16 or string is legal Java and reads with 0 refusals;
// the writer must then either refuse or write Go that compiles. It writes neither: the closure
// sorted_keys emits names the package sort, the package utf16 and the conversion string, all
// shadowed by the local.
func TestAdversarySortedKeysBesideALocalNamedLikeAPackageItUses(t *testing.T) {
	for _, local := range []string{"sort", "utf16", "string"} {
		t.Run(local, func(t *testing.T) {
			out := filepath.Join(scratch(t), "shadow")
			if err := run(strings.NewReader(shadowedSortedKeys(local)), out); err != nil {
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
		})
	}
}
