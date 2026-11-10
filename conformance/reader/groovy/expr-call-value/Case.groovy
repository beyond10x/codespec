package expr_call_value

/** The Groovy source of the kit case expr-call-value: Groovy 2.4 has no method reference, a method closure stands for one. */
final class Case {
    static boolean positive(long n) {
        return n > 0L
    }

    /** Calls the closure a rule holds in a field on n. */
    static boolean allowed(long n) {
        Rule r = new Rule(Case.&positive)
        return r.check.call(n)
    }
}

/** Holds a check as a closure. */
final class Rule {
    final Closure<Boolean> check

    Rule(Closure<Boolean> check) {
        this.check = check
    }
}
