package expr_method;

/** The Java source of the kit case expr-method. */
public final class Case {
    /** Adds two values through a method. */
    public static long sum(long x, long y) {
        Acc a = new Acc();
        a.add(x);
        a.add(y);
        return a.n;
    }
}

/** Accumulates. */
final class Acc {
    long n;

    void add(long v) {
        n = n + v;
    }
}
