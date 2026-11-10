package expr_func_value;

import java.util.function.LongUnaryOperator;

/**
 * The Java source of the kit case expr-func-value: a static method and a method bound to its
 * receiver, both used as values through method references.
 */
public final class Case {
    static long doubled(long n) {
        return n * 2L;
    }

    /** Applies doubled, then the method add of a counter of step k, to n. */
    public static long compose(long n, long k) {
        LongUnaryOperator f = Case::doubled;
        Counter c = new Counter(k);
        LongUnaryOperator g = c::add;
        return g.applyAsLong(f.applyAsLong(n));
    }
}

/** Adds its step. */
final class Counter {
    final long step;

    Counter(long step) {
        this.step = step;
    }

    long add(long n) {
        return n + step;
    }
}
