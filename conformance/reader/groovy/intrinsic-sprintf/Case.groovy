package intrinsic_sprintf;

/**
 * The Java source of the kit case intrinsic-sprintf. The name is formatted from a field: the
 * reader refuses a String parameter printed as it is, since Java prints a null one as "null".
 */
public final class Case {
    /** Formats a name and a count. */
    public static String label(String name, long n) {
        Tag t = new Tag(name);
        return t.format(n);
    }
}

/** A name to format. */
final class Tag {
    private final String name;

    Tag(String name) {
        this.name = name;
    }

    String format(long n) {
        return String.format("%s=%d", name, n);
    }
}
