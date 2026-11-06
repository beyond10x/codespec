package ledger;

/** The root of every error a ledger reports. */
public class LedgerException extends Exception {
    private static final long serialVersionUID = 1L;

    public LedgerException(String message) {
        super(message);
    }
}
