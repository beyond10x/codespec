package ledger;

/** An account opened under an id that is already open. */
public class DuplicateAccount extends LedgerException {
    private static final long serialVersionUID = 1L;

    public DuplicateAccount() {
        super("duplicate account");
    }
}
