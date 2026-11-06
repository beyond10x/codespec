package listalias;

import java.util.ArrayList;
import java.util.List;

/** A Java List is one shared object; every name for it sees an add. */
public class Fill {
    private final List<String> items = new ArrayList<>();

    /** Appends to the caller's list. */
    public static void fill(List<String> out) {
        out.add("x");
    }

    /** Java returns 1: fill added to l itself. */
    public static int filled() {
        List<String> l = new ArrayList<>();
        fill(l);
        return l.size();
    }

    /** Java returns 1: alias and items are one list. */
    public int aliased() {
        List<String> alias = items;
        alias.add("y");
        return items.size();
    }

    /** Java: false, two distinct lists; equal contents do not make them ==. */
    public static boolean same() {
        List<String> a = new ArrayList<>();
        List<String> b = new ArrayList<>();
        return a == b;
    }
}
