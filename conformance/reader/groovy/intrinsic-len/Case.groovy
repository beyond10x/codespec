package intrinsic_len;

import java.util.ArrayList;
import java.util.List;

/** The Java source of the kit case intrinsic-len. */
public final class Case {
    /** The size of a list of n values. */
    public static int count(long n) {
        List<String> xs = new ArrayList<>();
        for (long i = 0L; i < n; i++) {
            xs.add("x");
        }
        return xs.size();
    }
}
