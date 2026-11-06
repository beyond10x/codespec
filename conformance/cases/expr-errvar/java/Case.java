package expr_errvar;

/** The Java source of the kit case expr-errvar. */
public final class Case {
    /** Refuses the empty string. */
    public static void check(String s) throws CheckException {
        if (s.isEmpty()) {
            throw new Empty();
        }
    }
}

/** The root of every error check reports. */
class CheckException extends Exception {
    private static final long serialVersionUID = 1L;

    CheckException(String message) {
        super(message);
    }
}

/** The empty string. */
class Empty extends CheckException {
    private static final long serialVersionUID = 1L;

    Empty() {
        super("empty");
    }
}
