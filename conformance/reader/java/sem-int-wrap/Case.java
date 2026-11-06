package sem_int_wrap;

/** The Java source of the kit case sem-int-wrap: a long wraps as Go's int64 does. */
public final class Case {
    /** Adds, wrapping on overflow. */
    public static long add(long a, long b) {
        return a + b;
    }

    /** Multiplies, wrapping on overflow. */
    public static long mul(long a, long b) {
        return a * b;
    }

    /** Divides; Long.MIN_VALUE / -1 wraps to Long.MIN_VALUE. */
    public static long div(long a, long b) {
        return a / b;
    }

    /** The remainder; Long.MIN_VALUE % -1 is 0. */
    public static long rem(long a, long b) {
        return a % b;
    }

    /** Negates; -Long.MIN_VALUE wraps to Long.MIN_VALUE. */
    public static long neg(long a) {
        return -a;
    }
}
