package assign_or;

/** The Java source of the kit case assign-or. */
public final class Case {
    /** Ors in place. */
    public static long orWith(long a, long b) {
        a |= b;
        return a;
    }
}
