package shift_untyped;

/**
 * An int literal shifted by a count that is not constant, in a place that gives it no type. Java
 * shifts the int: {@code 1 << 31} is negative. The read is {@code binop << (lit 1 : int32)}; written
 * as Go, the untyped 1 takes the type the context gives it, {@code int} (64 bits) in a comparison
 * with a constant and in a {@code ...any} argument, and the shift is positive.
 */
public final class Case {
    /** Whether one shifted left n times is negative. */
    public static boolean negative(long n) {
        return (1 << n) < 0;
    }

    /** Prints one shifted left n times. */
    public static void show(long n) {
        System.out.println(1 << n);
    }

    /** One shifted left n times, in a string. */
    public static String text(long n) {
        return "v " + (1 << n);
    }
}
