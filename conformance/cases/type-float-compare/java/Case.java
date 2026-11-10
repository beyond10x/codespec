package type_float_compare;

/** The Java source of the kit case type-float-compare: double and float values and arithmetic. */
public final class Case {
    /**
     * Tells whether a tenth plus two tenths is three tenths: in doubles it is not, in floats the sum
     * rounds to three tenths.
     */
    public static boolean exact(boolean wide) {
        if (wide) {
            double x = 0.1;
            double y = 0.2;
            return x + y == 0.3;
        }
        float a = 0.1f;
        float b = 0.2f;
        return a + b == 0.3f;
    }

    /** Tells whether a third of one, or of 1e21, times three is the number again, in floats. */
    public static boolean third(boolean big) {
        float y = 1;
        if (big) {
            y = 1e21f;
        }
        return y / 3 * 3 == y;
    }
}
