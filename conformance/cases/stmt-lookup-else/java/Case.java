package stmt_lookup_else;

import java.util.HashMap;
import java.util.Map;

/**
 * The Java source of the kit case stmt-lookup-else: a local set by {@code Map.get} of a boxed value,
 * followed by an {@code if} on its null that ends in a throw, looks the key up and runs the {@code if}
 * when it is absent.
 */
public final class Case {
    /** Looks a key up in a map of two entries. */
    public static long find(String k) throws FindException {
        Map<String, Long> m = new HashMap<>();
        m.put("a", 1L);
        m.put("b", 2L);
        Long v = m.get(k);
        if (v == null) {
            throw new Missing();
        }
        return v;
    }
}

/** The root of every error find reports. */
class FindException extends Exception {
    private static final long serialVersionUID = 1L;

    FindException(String message) {
        super(message);
    }
}

/** The key is not in the map. */
class Missing extends FindException {
    private static final long serialVersionUID = 1L;

    Missing() {
        super("missing");
    }
}
