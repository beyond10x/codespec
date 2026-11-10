package expr_func_value

/** The Groovy source of the kit case expr-func-value: Groovy 2.4 has no method reference, method closures stand for them. */
final class Case {
    static long doubled(long n) {
        return n * 2L
    }

    /** Applies doubled, then the method add of a counter of step k, to n. */
    static long compose(long n, long k) {
        Closure<Long> f = Case.&doubled
        Counter c = new Counter(k)
        Closure<Long> g = c.&add
        return g.call(f.call(n))
    }
}

/** Adds its step. */
final class Counter {
    final long step

    Counter(long step) {
        this.step = step
    }

    long add(long n) {
        return n + step
    }
}
