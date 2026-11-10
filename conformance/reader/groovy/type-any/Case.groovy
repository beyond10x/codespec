package type_any;

/** The Java source of the kit case type-any: {@code Object} is the top type. */
public final class Case {
    /** Returns its integer as an Object. */
    public static Object boxed(long n) {
        return n;
    }
}
