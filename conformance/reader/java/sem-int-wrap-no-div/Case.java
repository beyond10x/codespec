package sem_int_wrap_no_div;

/**
 * The Java source of the kit case sem-int-wrap-no-div: a long wraps as Go's int64 does, without
 * the division the case sem-int-wrap also makes.
 */
public final class Case {
    /** Adds, wrapping on overflow. */
    public static long add(long a, long b) {
        return a + b;
    }

    /** Subtracts, wrapping on overflow. */
    public static long sub(long a, long b) {
        return a - b;
    }

    /** Multiplies, wrapping on overflow. */
    public static long mul(long a, long b) {
        return a * b;
    }

    /** Negates; -Long.MIN_VALUE wraps to Long.MIN_VALUE. */
    public static long neg(long a) {
        return -a;
    }
}
