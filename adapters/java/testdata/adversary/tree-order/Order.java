package treeorder;

import java.util.TreeMap;

/** A TreeMap iterates in its comparator's order, which is not always the order of the key's text. */
public class Order {
    /** Java returns "RED GREEN BLUE " (enum order is declaration order); sorted names give "BLUE GREEN RED ". */
    public static String byEnum() {
        TreeMap<Hue, Integer> m = new TreeMap<>();
        m.put(Hue.RED, 1);
        m.put(Hue.GREEN, 2);
        m.put(Hue.BLUE, 3);
        String out = "";
        for (Hue h : m.keySet()) {
            out += h + " ";
        }
        return out;
    }

    /** The caller built m, maybe with String.CASE_INSENSITIVE_ORDER or a reversed comparator. */
    public static String given(TreeMap<String, Integer> m) {
        String out = "";
        for (String k : m.keySet()) {
            out += k + " ";
        }
        return out;
    }
}
