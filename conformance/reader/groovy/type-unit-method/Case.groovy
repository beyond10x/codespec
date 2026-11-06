package type_unit_method;

/** The Java source of the kit case type-unit-method: a call of a void method is typed unit. */
public final class Case {
    /** Bumps a counter k times. */
    public static long count(long k) {
        Counter c = new Counter();
        for (long i = 0L; i < k; i++) {
            c.bump();
        }
        return c.n;
    }
}

/** Counts. */
final class Counter {
    long n;

    void bump() {
        n++;
    }
}
