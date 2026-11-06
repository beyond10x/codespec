package expr_intrinsic_slice;

import java.util.ArrayList;
import java.util.List;

/** The Java source of the kit case expr-intrinsic-slice: the size of a list is len of a slice. */
public final class Case {
    /** The size of a list holding n words. */
    public static int size(long n) {
        List<String> words = new ArrayList<>();
        for (long i = 0L; i < n; i++) {
            words.add("w");
        }
        return words.size();
    }
}
