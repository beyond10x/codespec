package javax.lending;

public class Overdue extends LendingException {
    private static final long serialVersionUID = 1L;

    public Overdue() {
        super("overdue");
    }
}
