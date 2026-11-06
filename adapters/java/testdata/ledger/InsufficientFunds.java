package ledger;

/** A withdrawal or fee larger than the balance. */
public class InsufficientFunds extends LedgerException {
    private static final long serialVersionUID = 1L;

    public InsufficientFunds() {
        super("insufficient funds");
    }
}
