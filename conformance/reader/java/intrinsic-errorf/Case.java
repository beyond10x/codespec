package intrinsic_errorf;

/**
 * The Java source of the kit case intrinsic-errorf: an exception whose one constructor formats its
 * message from its arguments with {@code String.format} is that formatted error.
 */
public final class Case {
    /** Refuses a negative. */
    public static void validate(long n) throws CheckException {
        if (n < 0L) {
            throw new Negative(n);
        }
    }
}

/** The root of every error the case reports. */
class CheckException extends Exception {
    private static final long serialVersionUID = 1L;

    CheckException(String message) {
        super(message);
    }
}

/** A negative value, named in the message. */
class Negative extends CheckException {
    private static final long serialVersionUID = 1L;

    Negative(long n) {
        super(String.format("negative: %d", n));
    }
}
