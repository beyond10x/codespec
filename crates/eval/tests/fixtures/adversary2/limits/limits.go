// Package limits holds a zero value at the evaluator's stated cell limit and a uint8 parameter.
package limits

// Big gives the length of an array of 4,194,304 ints, exactly the stated limit.
func Big() int {
	var a [4194304]int
	return len(a)
}

// U reports whether its uint8 argument is 255.
func U(x uint8) bool { return x == 255 }
