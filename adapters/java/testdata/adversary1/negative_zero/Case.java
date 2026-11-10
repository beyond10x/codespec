package negative_zero;

/**
 * Java's {@code -0.0} is negative zero, and one divided by it is negative infinity. The read is
 * {@code unop - (float lit 0.0)}; written as Go, {@code -0.0} is a constant, and a Go constant has
 * no negative zero.
 */
public final class Case {
    /** Whether one over negative zero is negative, in doubles. */
    public static boolean wide(boolean f) {
        double z = -0.0;
        double one = 1.0;
        return one / z < 0;
    }

    /** The same in floats. */
    public static boolean narrow(boolean f) {
        float z = -0.0f;
        float one = 1.0f;
        return one / z < 0;
    }
}
