package expr_struct;

/** The Java source of the kit case expr-struct. */
public final class Case {
    /** Builds a point and reads its x. */
    public static long make(long x) {
        Point p = new Point();
        p.x = x;
        return p.x;
    }
}

/** A position. */
final class Point {
    long x;
    long y;
}
