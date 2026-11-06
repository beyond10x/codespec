package ledger;

/** A posting of zero or less. */
public class NonPositiveAmount extends LedgerException {
    private static final long serialVersionUID = 1L;

    public NonPositiveAmount() {
        super("non-positive amount");
    }
}
