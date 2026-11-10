package assign_shr;

/** The Java source of the kit case assign-shr: {@code >>=}, its count masked as Java masks it. */
public final class Case {
    /** Shifts right in place. */
    public static long shrBy(long a, long b) {
        a >>= b;
        return a;
    }
}
