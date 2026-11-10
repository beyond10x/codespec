package decl_variadic_local;

/**
 * The Java source of the kit case decl-variadic-local: a variable arity method takes zero or more
 * arguments, which its body sees as one array.
 */
public final class Case {
    static long sum(long... xs) {
        long t = 0L;
        for (long x : xs) {
            t = t + x;
        }
        return t;
    }

    /** Calls sum with no, one and three arguments. */
    public static long sums(long a, long b) {
        return sum() + sum(a) + sum(a, b, 3L);
    }
}
