package assign_rem;

/** The Java source of the kit case assign-rem. */
public final class Case {
    /** Takes the remainder in place. */
    public static long remBy(long a, long b) {
        a %= b;
        return a;
    }
}
