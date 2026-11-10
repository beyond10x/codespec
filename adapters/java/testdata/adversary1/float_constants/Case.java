package float_constants;

/**
 * Arithmetic on float literals. Java rounds each operation to double or float; the read keeps
 * {@code binop + (float lit) (float lit)}, and written as Go that is a constant expression, which
 * Go evaluates exactly and rounds once.
 */
public final class Case {
    /** Whether 0.1 + 0.2, rounded as Java rounds it, is 0.3: it is not. */
    public static boolean sum(boolean f) {
        double a = 0.1 + 0.2;
        return a == 0.3;
    }

    /** Whether 1e16 + 1 + 1 is 1e16 in doubles: each + 1 rounds back to 1e16. */
    public static boolean chain(boolean f) {
        double a = 1e16 + 1.0 + 1.0;
        return a == 1e16;
    }

    /** The same in floats at 2^24. */
    public static boolean narrow(boolean f) {
        float a = 16777216.0f + 1.0f + 1.0f;
        return a == 16777216.0f;
    }
}
