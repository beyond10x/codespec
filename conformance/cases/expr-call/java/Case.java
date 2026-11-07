package expr_call;

/** The Java source of the kit case expr-call. */
public final class Case {
    private static long twice(long n) {
        return n + n;
    }

    /** Calls a module function twice. */
    public static long quad(long n) {
        return twice(twice(n));
    }
}
