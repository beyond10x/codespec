package intrinsic_sorted_keys;

import java.util.ArrayList;
import java.util.List;
import java.util.TreeMap;

/** The Java source of the kit case intrinsic-sorted-keys. */
public final class Case {
    /** Concatenates the distinct strings in ascending order. */
    public static String join(String a, String b, String c) {
        TreeMap<String, Long> m = new TreeMap<>();
        m.put(a, 1L);
        m.put(b, 2L);
        m.put(c, 3L);
        String s = "";
        for (String k : m.keySet()) {
            s += k;
        }
        return s;
    }

    /** Gives the distinct integers in ascending order. */
    public static List<Long> ints(long a, long b, long c) {
        TreeMap<Long, Long> m = new TreeMap<>();
        m.put(a, a);
        m.put(b, b);
        m.put(c, c);
        List<Long> keys = new ArrayList<>();
        for (Long k : m.keySet()) {
            keys.add(k);
        }
        return keys;
    }
}
