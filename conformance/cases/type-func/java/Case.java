package type_func;

import java.util.function.LongUnaryOperator;

/** The Java source of the kit case type-func: a parameter of a functional interface type. */
public final class Case {
    static long twice(long n) {
        return n * 2L;
    }

    static long run(LongUnaryOperator f, long n) {
        return f.applyAsLong(n);
    }

    /** Passes twice to run, whose parameter has a function type. */
    public static long apply(long n) {
        return run(Case::twice, n);
    }
}
