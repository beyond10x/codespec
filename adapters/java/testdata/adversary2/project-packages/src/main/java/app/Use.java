package app;

import app.lib.Box;
import app.lib.Chain;
import app.lib.Outer;
import app.lib.Thing;

public class Use {
    public Outer.Nested nested(Outer.Nested n) {
        return n;
    }

    public Chain chained() {
        return new Chain();
    }

    public Box boxed() {
        return new Box(5);
    }

    public static void bump(Thing t) {
        t.count = t.count + 1;
    }

    public static long twice(Thing t) {
        bump(t);
        bump(t);
        return t.count;
    }
}
