package forms;

import forms.other.Bag;
import forms.other.Thing;

public class Reach {
    public long count(Thing t) {
        return t.count; // refused: a class of another package
    }

    public boolean labelled(Thing t) {
        return t.label != null; // refused: a class of another package
    }

    public void bump(Thing t) {
        t.count = t.count + 1; // refused: a class of another package
    }

    public long size(Bag<String> b) {
        return b.size; // refused: a class of another package
    }
}
