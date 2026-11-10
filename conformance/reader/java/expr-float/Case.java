package expr_float;

/** The Java source of the kit case expr-float: double literals and double arithmetic. */
public final class Case {
    /** Tells whether a quarter of a float literal is more than half. */
    public static boolean over(boolean big) {
        double x = 0.25;
        if (big) {
            x = 1.5e300;
        }
        return x / 4 > 0.5;
    }
}
