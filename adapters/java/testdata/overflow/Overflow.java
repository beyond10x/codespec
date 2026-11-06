package overflow;

/** int arithmetic wraps at 32 bits, long arithmetic at 64. */
public class Overflow {
    public static int wrap() {
        return Integer.MAX_VALUE + 1;
    }

    public static long wide() {
        return Long.MAX_VALUE + 1L;
    }
}
