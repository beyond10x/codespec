package main

import (
	"go/constant"
	"go/types"
	"math"
	"strconv"
	"strings"
)

// floatBits is 32 or 64 for a float type (a named type over one included), else 0. An untyped
// float constant takes Go's default type, float64.
func floatBits(t types.Type) int {
	if t == nil {
		return 0
	}
	b, ok := t.Underlying().(*types.Basic)
	if !ok {
		return 0
	}
	switch b.Kind() {
	case types.Float32:
		return 32
	case types.Float64, types.UntypedFloat:
		return 64
	}
	return 0
}

// floatSpelling is the value of a cir/1 float literal for the constant v at width bits: the
// shortest decimal that reads back to the same number, plain when its decimal exponent e is
// -7 < e < 21, else <digits>e<e> without `+`; float_spelling in crates/model spells it the same
// way. ok is false when v is not a finite number of that width.
func floatSpelling(v constant.Value, bits int) (string, bool) {
	if v == nil {
		return "", false
	}
	v = constant.ToFloat(v)
	if v.Kind() != constant.Float {
		return "", false
	}
	var x float64
	if bits == 32 {
		x32, _ := constant.Float32Val(v)
		x = float64(x32)
	} else {
		x, _ = constant.Float64Val(v)
	}
	if math.IsInf(x, 0) || math.IsNaN(x) {
		return "", false
	}
	sci := strconv.FormatFloat(x, 'e', -1, bits)
	mantissa, exp, _ := strings.Cut(sci, "e")
	e, err := strconv.Atoi(exp)
	if err != nil {
		return "", false
	}
	if e <= -7 || e >= 21 {
		return mantissa + "e" + strconv.Itoa(e), true
	}
	sign := ""
	if strings.HasPrefix(mantissa, "-") {
		sign, mantissa = "-", mantissa[1:]
	}
	digits := strings.Replace(mantissa, ".", "", 1)
	point := e + 1
	switch {
	case point <= 0:
		return sign + "0." + strings.Repeat("0", -point) + digits, true
	case len(digits) <= point:
		return sign + digits + strings.Repeat("0", point-len(digits)), true
	}
	return sign + digits[:point] + "." + digits[point:], true
}
