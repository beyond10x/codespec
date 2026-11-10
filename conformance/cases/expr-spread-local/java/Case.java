package expr_spread_local;

/**
 * The Java source of the kit case expr-spread-local: an array passed to a variable arity method is
 * the array the method sees, as a slice passed spread is.
 */
public final class Case {
    static long sum(long... xs) {
        long t = 0L;
        for (long x : xs) {
            t = t + x;
        }
        return t;
    }

    /** Builds an array of a, b, a and b and passes it to sum: 2*(a+b). */
    public static long twice(long a, long b) {
        long[] xs = {a, b, a, b};
        return sum(xs);
    }
}
