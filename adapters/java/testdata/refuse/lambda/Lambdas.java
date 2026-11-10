package lambdas;

import java.util.function.UnaryOperator;

public class Lambdas {
    public static int twice(int x) {
        UnaryOperator<Integer> f = y -> y * 2; // refused
        return f.apply(x);
    }
}
