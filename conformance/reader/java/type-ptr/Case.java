package type_ptr;

/** The Java source of the kit case type-ptr: an object is reached through a reference. */
public final class Case {
    /** Increments a value through a reference to the cell holding it. */
    public static long bump(long n) {
        Cell c = new Cell(n);
        Cell p = c;
        p.v = p.v + 1L;
        return c.v;
    }
}

/** Holds a value. */
final class Cell {
    long v;

    Cell(long v) {
        this.v = v;
    }
}
