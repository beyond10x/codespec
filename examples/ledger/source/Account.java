package ledger;

import java.util.ArrayList;
import java.util.List;

/** One account: its balance, its points, how many postings it took and their amounts. */
public class Account {
    private final String id;
    private long balance;
    private int points;
    private int postings;
    private final List<Long> amounts = new ArrayList<>();

    public Account(String id) {
        this.id = id;
    }

    public String id() {
        return id;
    }

    public long balance() {
        return balance;
    }

    public int points() {
        return points;
    }

    public int postings() {
        return postings;
    }

    /** Records one amount: a deposit adds it, a withdrawal or a fee takes it, a withdrawal never past the balance; the balance wraps as a long does. */
    public void post(Kind kind, long amount) throws LedgerException {
        if (kind == Kind.WITHDRAWAL && balance < amount) {
            throw new InsufficientFunds();
        }
        switch (kind) {
            case DEPOSIT:
                balance += amount;
                break;
            case WITHDRAWAL:
            case FEE:
                balance -= amount;
                break;
            default:
                break;
        }
        postings++;
        amounts.add(amount);
    }

    /** Adds n points, which may be negative; the total wraps as an int does. */
    public int award(int n) {
        points += n;
        return points;
    }

    /** The largest single amount, or 0. */
    public long largest() {
        long max = 0;
        for (int i = 0; i < amounts.size(); i++) {
            if (amounts.get(i) > max) {
                max = amounts.get(i);
            }
        }
        return max;
    }

    /** How many leading amounts it takes to reach target, or -1. */
    public int reach(long target) {
        int n = 0;
        long sum = 0;
        while (n < amounts.size()) {
            sum += amounts.get(n);
            n++;
            if (sum >= target) {
                break;
            }
        }
        if (sum < target) {
            return -1;
        }
        return n;
    }

    public String describe() {
        return String.format("%s: %d after %d postings, %d points", id, balance, postings, points);
    }
}
