package sem_error_propagate;

/**
 * The Java source of the kit case sem-error-propagate: a call of a method that throws, not caught,
 * passes the callee's exception on to the caller unchanged.
 */
public final class Case {
    /** n, or the sentinel error when n is negative. */
    public static long checked(long n) throws CheckException {
        if (n < 0L) {
            throw new Negative();
        }
        return n;
    }

    /** Refuses a negative. */
    public static void valid(long n) throws CheckException {
        if (n < 0L) {
            throw new Negative();
        }
    }

    /** Passes checked's exception on unchanged, or returns its value. */
    public static long through(long n) throws CheckException {
        long v = checked(n);
        return v;
    }

    /** Passes valid's exception on unchanged, or returns n. */
    public static long guarded(long n) throws CheckException {
        valid(n);
        return n;
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
