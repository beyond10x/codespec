package expr_const;

/** The Java source of the kit case expr-const. */
public final class Case {
    /** The largest value clamp returns. */
    public static final long LIMIT = 10L;

    /** Caps n at LIMIT. */
    public static long clamp(long n) {
        if (n > LIMIT) {
            return LIMIT;
        }
        return n;
    }
}
