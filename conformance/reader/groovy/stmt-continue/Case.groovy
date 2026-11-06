package stmt_continue;

import java.util.ArrayList;
import java.util.List;

/** The Java source of the kit case stmt-continue. */
public final class Case {
    /** Sums the values up to n-1 except skip. */
    public static long skipSum(long n, long skip) {
        List<Long> xs = new ArrayList<>();
        for (long i = 0L; i < n; i++) {
            xs.add(i);
        }
        long s = 0L;
        for (long x : xs) {
            if (x == skip) {
                continue;
            }
            s += x;
        }
        return s;
    }
}
