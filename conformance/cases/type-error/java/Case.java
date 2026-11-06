package type_error;

/** The Java source of the kit case type-error. */
public final class Case {
    /** Fails when told to. */
    public static void fail(boolean bad) throws FailException {
        if (bad) {
            throw new Bad();
        }
    }
}

/** The root of every error fail reports. */
class FailException extends Exception {
    private static final long serialVersionUID = 1L;

    FailException(String message) {
        super(message);
    }
}

/** Told to fail. */
class Bad extends FailException {
    private static final long serialVersionUID = 1L;

    Bad() {
        super("bad");
    }
}
