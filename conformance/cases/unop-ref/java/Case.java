package unop_ref;

/** The Java source of the kit case unop-ref. */
public final class Case {
    /** Reads a value through a reference to a fresh box. */
    public static long boxed(long v) {
        Box b = new Box(v);
        return b.v;
    }
}

/** Holds a value. */
final class Box {
    long v;

    Box(long v) {
        this.v = v;
    }
}
