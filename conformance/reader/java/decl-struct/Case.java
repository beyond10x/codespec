package decl_struct;

/** The Java source of the kit case decl-struct. */
public final class Case {
    /** The width of a span. */
    public static long width(long from, long to) {
        Span s = new Span(from, to);
        return s.to - s.from;
    }
}

/** A range. */
final class Span {
    long from;
    long to;

    Span(long from, long to) {
        this.from = from;
        this.to = to;
    }
}
