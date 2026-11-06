package lambdas;

import java.util.function.IntUnaryOperator;

public class Lambdas {
    public static int twice(int x) {
        IntUnaryOperator f = y -> y * 2; // refused
        return f.applyAsInt(x);
    }
}
