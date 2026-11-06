package ledger;

/** A posting to an account that was never opened. */
public class UnknownAccount extends LedgerException {
    private static final long serialVersionUID = 1L;

    public UnknownAccount() {
        super("unknown account");
    }
}
