package type_tuple_error;

/**
 * The Java source of the kit case type-tuple-error: a call of a method that returns a value and
 * throws is of the tuple of the value and the error.
 */
public final class Case {
    static long doubled(long n) throws CheckException {
        if (n < 0L) {
            throw new Negative();
        }
        return n * 2L;
    }

    /** Doubles n twice, or passes the error on. */
    public static long quadruple(long n) throws CheckException {
        long d = doubled(n);
        long q = doubled(d);
        return q;
    }
}

/** The root of every error the case reports. */
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
