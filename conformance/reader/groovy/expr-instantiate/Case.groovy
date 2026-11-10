package expr_instantiate;

/** The Java source of the kit case expr-instantiate: a call of a generic method instantiated at Long. */
public final class Case {
    static <T> T same(T v) {
        return v;
    }

    /** Passes n through a method over any type. */
    public static long wrap(long n) {
        return same(n);
    }
}
