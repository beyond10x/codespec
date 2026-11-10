package expr_spread_local

/** The Groovy source of the kit case expr-spread-local: Groovy 2.4 has no array initializer, a list coerced to an array stands for one. */
final class Case {
    static long sum(long... xs) {
        long t = 0L
        for (long x : xs) {
            t = t + x
        }
        return t
    }

    /** Builds an array of a, b, a and b and passes it to sum: 2*(a+b). */
    static long twice(long a, long b) {
        long[] xs = [a, b, a, b] as long[]
        return sum(xs)
    }
}
