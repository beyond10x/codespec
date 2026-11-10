package println_constant;

/**
 * An int constant expression that wraps, printed. Java folds {@code 2147483647 + 1} as an int; the
 * read is {@code binop + (lit : int32) (lit : int32)}, which Go prints as an untyped constant of
 * type {@code int}. {@code println} is new in the unit; the concatenation reads the same way at the
 * unit's base.
 */
public final class Case {
    /** Prints the int maximum plus one. */
    public static void show(long n) {
        System.out.println(2147483647 + 1);
    }

    /** The int maximum plus one, in a string. */
    public static String text(long n) {
        return "v " + (2147483647 + 1);
    }
}
