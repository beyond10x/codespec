package expr_lambda

/** The Groovy source of the kit case expr-lambda: Groovy 2.4 has no lambda, a closure stands for one. */
final class Case {
    /** Multiplies n by k twice through a closure and adds how often the closure ran. */
    static long scale(long n, long k) {
        Calls calls = new Calls()
        Closure<Long> mul = { long x ->
            calls.n++
            return x * k
        }
        long r = mul.call(mul.call(n))
        return r + calls.n
    }
}

/** Counts calls. */
final class Calls {
    long n
}
