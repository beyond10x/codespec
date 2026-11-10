package error_struct;

import java.util.List;

/**
 * An exception carrying a list: its message reads the list when it is asked for, so an element
 * the caller adds after the throw changes it, where a cir/1 slice copied into an error struct does
 * not grow with it.
 */
public final class Keys {
    /** Never finds the keys. */
    public static void get(List<String> ks) throws GetException {
        throw new NotFound(ks); // refused
    }

    /** The method beside them. */
    public static long one() {
        return 1L;
    }
}

/** The root of every error get reports. */
class GetException extends Exception {
    private static final long serialVersionUID = 1L;

    GetException(String message) {
        super(message);
    }
}

/** An error naming how many keys were missing. */
class NotFound extends GetException { // refused
    private static final long serialVersionUID = 1L;

    private final List<String> keys;

    NotFound(List<String> keys) {
        super("");
        this.keys = keys;
    }

    @Override
    public String getMessage() {
        return "not found: " + keys.size();
    }
}
