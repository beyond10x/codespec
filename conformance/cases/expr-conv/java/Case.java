package expr_conv;

import java.util.ArrayList;
import java.util.List;

/** The Java source of the kit case expr-conv: size() is a len converted to int. */
public final class Case {
    /** n, as the size of a list of n values. */
    public static int wrap(long n) {
        List<Long> xs = new ArrayList<>();
        for (long i = 0L; i < n; i++) {
            xs.add(i);
        }
        return xs.size();
    }
}
