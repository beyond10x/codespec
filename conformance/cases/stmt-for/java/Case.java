package stmt_for;

/** The Java source of the kit case stmt-for. */
public final class Case {
    /** Sums 0 up to n-1. */
    public static long below(long n) {
        long s = 0L;
        for (long i = 0L; i < n; i++) {
            s += i;
        }
        return s;
    }
}
