package sharedfield;

import java.util.ArrayList;
import java.util.List;

/** A field that holds a list another name also holds: an add through the field is seen through that name. */
public class Bag {
    private final List<String> items;
    private List<String> held = new ArrayList<>();

    public Bag(List<String> items) {
        this.items = items;
    }

    public void push(String s) {
        this.items.add(s);
    }

    /** Java returns 1: the bag holds the caller's list l itself. */
    public static int shared() {
        List<String> l = new ArrayList<>();
        Bag b = new Bag(l);
        b.push("x");
        return l.size();
    }

    /** Java returns 1: held and l are one list. */
    public int adopt() {
        List<String> l = new ArrayList<>();
        this.held = l;
        this.held.add("z");
        return l.size();
    }
}
