package op_shl;

/**
 * The Java source of the kit case op-shl: Java masks a long's shift count to its low six bits,
 * which the reader writes out as {@code b & 63}.
 */
public final class Case {
    /** Shifts left. */
    public static long shl(long a, long b) {
        return a << b;
    }
}
