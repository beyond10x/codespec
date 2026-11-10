package type_param;

/** The Java source of the kit case type-param: parameters and a result of a type variable. */
public final class Case {
    static <T> T latter(T x, T y) {
        return y;
    }

    /** Gives the second of two numbers through a method over any type. */
    public static long second(long a, long b) {
        return latter(a, b);
    }
}
