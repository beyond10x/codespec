package expr_iface_call;

/**
 * The Java source of the kit case expr-iface-call: a call through an interface of the package
 * with two implementations runs the method of the object it holds.
 */
public final class Case {
    static String emit(Echo e, String s) {
        return e.echo(s);
    }

    /** Echoes s twice through an Echo that gives it back unchanged. */
    public static String twice(String s) {
        Echo e = new Same();
        return emit(e, s) + emit(e, s);
    }
}

/** Gives a string back. */
interface Echo {
    String echo(String s);
}

final class Same implements Echo {
    public String echo(String s) {
        return s;
    }
}

final class Blank implements Echo {
    public String echo(String s) {
        return "";
    }
}
