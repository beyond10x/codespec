// Command reference runs the calls of one conformance case against its Go reference program and
// prints one line per call.
//
// The case's package is imported as codespec.conformance/case/subject, and calls.go, generated
// by codespec-conformance from the case's calls.json, defines run(): each call in order, its
// results handed to none, values or fallible, then each channel it was given drained. The output encoding is defined in the crate doc of
// crates/conformance.
package main

import (
	"fmt"
	"os"
	"reflect"
	"strconv"
	"strings"
)

func main() {
	run()
}

// none prints the line of a call that returns nothing.
func none() {
	fmt.Println("ok")
}

// values prints the line of a call whose results end in no error.
func values(vs ...any) {
	line := "ok"
	for _, v := range vs {
		line += " " + show(reflect.ValueOf(v))
	}
	fmt.Println(line)
}

// fallible prints the line of a call whose last result is an error.
func fallible(vs ...any) {
	if err, _ := vs[len(vs)-1].(error); err != nil {
		fmt.Println("err " + err.Error())
		return
	}
	values(vs[:len(vs)-1]...)
}

// drain closes a channel the call was given and prints one line per value sent on it, in order.
func drain[T any](ch chan T) {
	close(ch)
	for v := range ch {
		fmt.Println("sent " + show(reflect.ValueOf(v)))
	}
}

func show(v reflect.Value) string {
	switch v.Kind() {
	case reflect.Int, reflect.Int8, reflect.Int16, reflect.Int32, reflect.Int64:
		return strconv.FormatInt(v.Int(), 10)
	case reflect.Bool:
		return strconv.FormatBool(v.Bool())
	case reflect.String:
		return strconv.Quote(v.String())
	case reflect.Slice:
		parts := make([]string, v.Len())
		for i := range parts {
			parts[i] = show(v.Index(i))
		}
		return "[" + strings.Join(parts, " ") + "]"
	}
	fmt.Fprintf(os.Stderr, "reference: a result of kind %s cannot be printed\n", v.Kind())
	os.Exit(2)
	return ""
}
