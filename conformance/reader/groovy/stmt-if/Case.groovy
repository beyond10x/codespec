package stmt_if;

/** The Java source of the kit case stmt-if. */
public final class Case {
    /** 1 for a positive, 2 for a negative and 0 for zero. */
    public static long sign(long n) {
        if (n > 0L) {
            return 1L;
        } else if (n < 0L) {
            return 2L;
        }
        return 0L;
    }
}
