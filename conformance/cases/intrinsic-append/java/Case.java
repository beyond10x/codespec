package intrinsic_append;

import java.util.ArrayList;
import java.util.List;

/** The Java source of the kit case intrinsic-append. */
public final class Case {
    /** Collects 0 up to n-1. */
    public static List<Long> build(long n) {
        List<Long> xs = new ArrayList<>();
        for (long i = 0L; i < n; i++) {
            xs.add(i);
        }
        return xs;
    }
}
