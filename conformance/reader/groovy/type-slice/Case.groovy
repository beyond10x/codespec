package type_slice;

import java.util.ArrayList;
import java.util.List;

/** The Java source of the kit case type-slice. */
public final class Case {
    /** A list of two strings. */
    public static List<String> pair(String a, String b) {
        List<String> xs = new ArrayList<>();
        xs.add(a);
        xs.add(b);
        return xs;
    }
}
