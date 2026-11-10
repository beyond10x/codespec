package forms;

import forms.other.Bag;
import forms.other.Thing;

public class Reach {
    public long count(Thing t) {
        return t.count;
    }

    public boolean labelled(Thing t) {
        return t.label != null;
    }

    public long size(Bag<String> b) {
        return b.size; // refused: which its package does not read
    }
}
