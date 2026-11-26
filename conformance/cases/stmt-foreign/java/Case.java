package stmt_foreign;

/** The Java source of the kit case stmt-foreign. */
public final class Case {
    /**
     * Adds n to a total. A negative n first passes a try statement, which cir/1 cannot express;
     * the calls never take that path.
     */
    public static long count(long n) {
        long total = 0L;
        if (n < 0L) {
            try {
                total -= 1L;
            } catch (RuntimeException e) {
                total = 0L;
            }
        }
        total += n;
        return total;
    }
}
