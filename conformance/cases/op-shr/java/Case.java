package op_shr;

/**
 * The Java source of the kit case op-shr: an arithmetic right shift, its count masked to six bits
 * as Java masks a long's.
 */
public final class Case {
    /** Shifts right, arithmetically. */
    public static long shr(long a, long b) {
        return a >> b;
    }
}
