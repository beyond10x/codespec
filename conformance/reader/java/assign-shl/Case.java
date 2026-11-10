package assign_shl;

/** The Java source of the kit case assign-shl: {@code <<=}, its count masked as Java masks it. */
public final class Case {
    /** Shifts left in place. */
    public static long shlBy(long a, long b) {
        a <<= b;
        return a;
    }
}
