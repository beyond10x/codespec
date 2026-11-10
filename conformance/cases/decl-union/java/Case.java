package decl_union;

/**
 * The Java source of the kit case decl-union: a sealed interface whose permitted classes are
 * records is a tagged union, declared beside a function to call.
 */
public final class Case {
    /** Echo returns its argument. */
    public static long echo(long n) {
        return n;
    }
}

/** Shape is a circle, a rectangle or a label. */
sealed interface Shape permits Circle, Rect, Label {}

record Circle(long radius) implements Shape {}

record Rect(long w, long h) implements Shape {}

record Label(String text) implements Shape {}
