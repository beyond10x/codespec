package decl_enum_string;

/** The Java source of the kit case decl-enum-string. */
public final class Case {
    /** Red for zero and Green otherwise. */
    public static Color pick(long n) {
        if (n == 0L) {
            return Color.Red;
        }
        return Color.Green;
    }
}

/** One of two colours. */
enum Color {
    Red,
    Green
}
