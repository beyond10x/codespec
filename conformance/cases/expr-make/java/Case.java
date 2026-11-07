package expr_make;

import java.util.HashMap;
import java.util.Map;

/** The Java source of the kit case expr-make. */
public final class Case {
    /** Counts the entries of an empty map. */
    public static int fresh() {
        Map<String, Long> m = new HashMap<>();
        return m.size();
    }
}
