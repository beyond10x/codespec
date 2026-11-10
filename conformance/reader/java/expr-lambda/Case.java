package expr_lambda;

import java.util.function.LongUnaryOperator;

/**
 * The Java source of the kit case expr-lambda: a lambda capturing a parameter and a local. Java
 * captures only effectively final locals, so the count the lambda keeps is a field of an object
 * the captured local refers to.
 */
public final class Case {
    /** Multiplies n by k twice through a lambda and adds how often the lambda ran. */
    public static long scale(long n, long k) {
        Calls calls = new Calls();
        LongUnaryOperator mul = x -> {
            calls.n++;
            return x * k;
        };
        long r = mul.applyAsLong(mul.applyAsLong(n));
        return r + calls.n;
    }
}

/** Counts calls. */
final class Calls {
    long n;
}
