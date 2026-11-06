package growinloop;

import java.util.ArrayList;
import java.util.List;
import java.util.TreeMap;

/** A collection changed inside a for-each over itself: Java throws ConcurrentModificationException. */
public class Grow {
    private final List<String> items = new ArrayList<>();

    /** Java throws once items holds anything; a range over a slice runs to its end. */
    public void twice() {
        for (String s : this.items) {
            this.items.add(s);
        }
    }

    /** Java throws: the add moves the list under its own iterator. */
    public static int local() {
        List<String> l = new ArrayList<>();
        l.add("a");
        for (String s : l) {
            l.add(s);
        }
        return l.size();
    }

    /** Java throws: "a!" lands before "b", the next key, under the iterator; sorted_keys is a copy. */
    public static int keys() {
        TreeMap<String, Integer> m = new TreeMap<>();
        m.put("a", 1);
        m.put("b", 1);
        for (String k : m.keySet()) {
            m.put(k + "!", 2);
        }
        return m.size();
    }
}
