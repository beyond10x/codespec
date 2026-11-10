package lookup_else;

import java.util.HashMap;
import java.util.Map;

/**
 * A Map.get of a boxed value whose if on its null goes on after it: a cir/1 lookup_else runs its
 * else branch to a return, and has no form for a lookup that falls through with another value.
 */
public final class Find {
    /** The value under k, or zero. */
    public static long orZero(String k) {
        Map<String, Long> m = new HashMap<>();
        m.put("a", 1L);
        Long v = m.get(k); // refused
        if (v == null) { // refused
            v = 0L;
        }
        return v;
    }

    /** The method beside it. */
    public static long one() {
        return 1L;
    }
}
