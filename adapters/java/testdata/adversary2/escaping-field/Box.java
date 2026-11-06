package escapingfield;

import java.util.ArrayList;
import java.util.List;

/** A field built here and handed out by a getter: the caller's name sees every later add. */
public class Box {
    private final List<String> items = new ArrayList<>();

    public void push(String s) {
        this.items.add(s);
    }

    public List<String> view() {
        return this.items;
    }

    /** Java returns 1: v is the box's own list, so the push is seen through it. */
    public static int viewed() {
        Box b = new Box();
        List<String> v = b.view();
        b.push("y");
        return v.size();
    }
}
