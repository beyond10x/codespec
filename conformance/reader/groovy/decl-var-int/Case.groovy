package decl_var_int;

/**
 * The Java source of the kit case decl-var-int: static fields that are not constants are
 * module-level variables, one with a literal initializer and one at its zero value.
 */
public final class Case {
    /** Where every count starts. */
    static long start = 10L;

    /** Counts what hit was given; it starts at its zero value. */
    static long hits;

    /** Adds n to the count and gives the start plus the count. */
    public static long hit(long n) {
        hits = hits + n;
        return start + hits;
    }

    /** The count. */
    public static long calls() {
        return hits;
    }
}
