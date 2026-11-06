package expr_zero_error;

/** The Java source of the kit case expr-zero-error: a throw returns the zero value beside its error. */
public final class Case {
    /** n, or the sentinel error when n is negative. */
    public static long check(long n) throws CheckException {
        if (n < 0L) {
            throw new Negative();
        }
        return n;
    }
}

/** The root of every error check reports. */
class CheckException extends Exception {
    private static final long serialVersionUID = 1L;

    CheckException(String message) {
        super(message);
    }
}

/** A negative value. */
class Negative extends CheckException {
    private static final long serialVersionUID = 1L;

    Negative() {
        super("negative");
    }
}
