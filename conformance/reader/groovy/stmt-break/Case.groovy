package stmt_break;

/** The Java source of the kit case stmt-break. */
public final class Case {
    /** The smallest i with i*i > n. */
    public static long root(long n) {
        long i = 0L;
        while (true) {
            if (i * i > n) {
                break;
            }
            i++;
        }
        return i;
    }
}
