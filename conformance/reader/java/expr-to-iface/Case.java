package expr_to_iface;

/**
 * The Java source of the kit case expr-to-iface: an object bound to a local of an interface type,
 * the interface having two implementations in the package.
 */
public final class Case {
    /** Converts a Tag holding s to a Labeler and calls through it. */
    public static String label(String s) {
        Labeler l = new Tag(s);
        return l.label();
    }
}

interface Labeler {
    String label();
}

final class Tag implements Labeler {
    final String text;

    Tag(String text) {
        this.text = text;
    }

    public String label() {
        return "<" + text + ">";
    }
}

final class Plain implements Labeler {
    public String label() {
        return "plain";
    }
}
