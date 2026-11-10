package expr_to_error;

/**
 * The Java source of the kit case expr-to-error: an exception carrying a field, set by its
 * constructor, whose message {@code getMessage()} computes from it, is an error struct, and a throw
 * of it is the struct as an error.
 */
public final class Case {
    /** Never finds a key. */
    public static void get(String k) throws GetException {
        throw new NotFound(k);
    }
}

/** The root of every error get reports. */
class GetException extends Exception {
    private static final long serialVersionUID = 1L;

    GetException(String message) {
        super(message);
    }
}

/** An error naming a key. */
class NotFound extends GetException {
    private static final long serialVersionUID = 1L;

    private final String key;

    NotFound(String key) {
        super(""); // never read: getMessage() gives the message
        this.key = key;
    }

    @Override
    public String getMessage() {
        return "not found: " + key;
    }
}
