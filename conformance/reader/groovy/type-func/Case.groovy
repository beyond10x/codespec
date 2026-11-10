package type_func

/** The Groovy source of the kit case type-func: Groovy 2.4 has no method reference, a method closure stands for one. */
final class Case {
    static long twice(long n) {
        return n * 2L
    }

    static long run(Closure<Long> f, long n) {
        return f.call(n)
    }

    /** Passes twice to run, whose parameter is a closure. */
    static long apply(long n) {
        return run(Case.&twice, n)
    }
}
