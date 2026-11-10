package expr_global;

/** The Java source of the kit case expr-global: a static field read and assigned. */
public final class Case {
    static long total;

    /** Adds n to the running total and returns the new total. */
    public static long add(long n) {
        total = total + n;
        return total;
    }
}
