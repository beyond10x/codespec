package expr_index;

import java.util.ArrayList;
import java.util.List;

/** The Java source of the kit case expr-index. */
public final class Case {
    /** Indexes a list of two values. */
    public static long at(int i) {
        List<Long> xs = new ArrayList<>();
        xs.add(10L);
        xs.add(20L);
        return xs.get(i);
    }
}
