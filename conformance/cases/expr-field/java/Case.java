package expr_field;

/** The Java source of the kit case expr-field. */
public final class Case {
    /** The b of a pair. */
    public static long right(long a, long b) {
        Pair p = new Pair(a, b);
        return p.b;
    }
}

/** Two values. */
final class Pair {
    long a;
    long b;

    Pair(long a, long b) {
        this.a = a;
        this.b = b;
    }
}
