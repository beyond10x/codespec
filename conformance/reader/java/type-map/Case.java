package type_map;

import java.util.HashMap;
import java.util.Map;

/** The Java source of the kit case type-map. */
public final class Case {
    /** Puts 3 under s and reads it back. */
    public static long stored(String s) {
        Map<String, Long> m = new HashMap<>();
        m.put(s, 3L);
        return m.getOrDefault(s, 0L);
    }
}
