package sem_map_order;

import java.util.HashMap;

/** The Java source of the kit case sem-map-order. */
public final class Case {
    /** Sums the squares of 0 up to n-1 through a map; the sum is the same in any order. */
    public static long squares(long n) {
        HashMap<Long, Long> m = new HashMap<>();
        for (long i = 0L; i < n; i++) {
            m.put(i, i * i);
        }
        long s = 0L;
        for (Long k : m.keySet()) {
            s += m.getOrDefault(k, 0L);
        }
        return s;
    }
}
