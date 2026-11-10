package expr_call_value;

import java.util.function.LongPredicate;

/** The Java source of the kit case expr-call-value: a call of a function value held in a field. */
public final class Case {
    static boolean positive(long n) {
        return n > 0L;
    }

    /** Calls the function value a rule holds in a field on n. */
    public static boolean allowed(long n) {
        Rule r = new Rule(Case::positive);
        return r.check.test(n);
    }
}

/** Holds a check as a function value. */
final class Rule {
    final LongPredicate check;

    Rule(LongPredicate check) {
        this.check = check;
    }
}
