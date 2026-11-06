package stmt_for_range;

import java.util.ArrayList;
import java.util.List;

/** The Java source of the kit case stmt-for-range. */
public final class Case {
    /** Sums the first n naturals collected in a list. */
    public static long total(long n) {
        List<Long> xs = new ArrayList<>();
        for (long i = 0L; i < n; i++) {
            xs.add(i);
        }
        long s = 0L;
        for (long x : xs) {
            s += x;
        }
        return s;
    }
}
